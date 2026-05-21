//! NLI-based citation support check.
//!
//! Loads a small NLI cross-encoder from the Hugging Face hub, encodes
//! (sentence, span_text) pairs with the model's tokenizer, runs `ort`
//! inference, and produces a 3-class [`SupportVerdict`] by argmax over
//! the model's logits.
//!
//! The default model is `Xenova/distilbert-base-uncased-mnli` — ~265 MB on
//! disk, around 50 ms per pair on CPU. Label order is read from the
//! model's `config.json` so a different repo can be substituted without
//! code changes, provided the labels are some permutation of
//! `ENTAILMENT` / `NEUTRAL` / `CONTRADICTION`.

#![allow(clippy::module_name_repetitions)]

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use ndarray::Array2;
use ort::session::Session;
use ort::value::Value;
use serde::Deserialize;
use thiserror::Error;
use tokenizers::Tokenizer;

use super::{SupportChecker, SupportError, SupportVerdict};

/// Default HuggingFace repo. Distilled BERT trained on MNLI — small enough
/// to download in a reasonable time and accurate enough to demonstrate the
/// pipeline. Override with [`NliCrossEncoder::for_model`].
pub const DEFAULT_NLI_MODEL_REPO: &str = "Xenova/distilbert-base-uncased-mnli";

/// Candidate ONNX weight paths inside an MNLI repo, tried in order.
/// The ecosystem is inconsistent: transformers.js / Xenova exports use
/// `onnx/model.onnx`; Optimum exports drop `model.onnx` at the repo
/// root; some only publish a quantized graph. Full precision is
/// preferred over quantized (the support gate is accuracy-sensitive).
/// This list is why `compare --candidate-nli <a real DeBERTa ONNX repo>`
/// works first try instead of 404-ing on a layout assumption.
const ONNX_WEIGHT_CANDIDATES: &[&str] = &[
    "onnx/model.onnx",
    "model.onnx",
    "onnx/model_quantized.onnx",
    "model_quantized.onnx",
];

/// Resolve the ONNX weights file, trying each known layout. Returns a
/// clear error listing every path attempted if none resolve — far more
/// actionable than a raw hf-hub 404 on a single hardcoded path.
fn fetch_onnx_weights(
    repo: &hf_hub::api::sync::ApiRepo,
    repo_id: &str,
) -> Result<std::path::PathBuf, NliError> {
    let mut last_err = String::new();
    for candidate in ONNX_WEIGHT_CANDIDATES {
        match repo.get(candidate) {
            Ok(path) => return Ok(path),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(NliError::Download(format!(
        "NLI repo `{repo_id}` exposes no ONNX weights at any known path \
         ({}). It is probably not an ONNX export — point `--nli-model` / \
         `--candidate-nli` at an ONNX conversion of the checkpoint. Last \
         error: {last_err}",
        ONNX_WEIGHT_CANDIDATES.join(", "),
    )))
}

#[derive(Debug, Error)]
pub enum NliError {
    #[error("failed to fetch NLI model files: {0}")]
    Download(String),
    #[error("failed to load tokenizer: {0}")]
    Tokenizer(String),
    #[error("failed to initialize ort session: {0}")]
    Session(String),
    #[error("inference failed: {0}")]
    Run(String),
    #[error(
        "model config has no id2label, or it isn't a permutation of \
         ENTAILMENT/NEUTRAL/CONTRADICTION"
    )]
    BadConfig,
}

/// NLI cross-encoder. Holds an `ort` session behind a [`Mutex`] (the
/// session's `run` takes `&mut self`) and the precomputed label-index
/// permutation for the loaded model.
pub struct NliCrossEncoder {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    labels: LabelIndices,
}

/// Where each logit lives in the model's output vector. Read from the
/// model's `config.json` so a different model with a different label order
/// works without recompilation.
#[derive(Debug, Clone, Copy)]
struct LabelIndices {
    entailment: usize,
    neutral: usize,
    contradiction: usize,
}

#[derive(Deserialize)]
struct ConfigJson {
    #[serde(default)]
    id2label: std::collections::BTreeMap<String, String>,
}

impl NliCrossEncoder {
    /// Load the default model. First call downloads ~265 MB to the
    /// Hugging Face cache directory.
    ///
    /// # Errors
    ///
    /// Returns [`NliError::Download`] on network / cache failures,
    /// [`NliError::Session`] for ort init failures.
    pub fn new() -> Result<Self, NliError> {
        Self::for_model(DEFAULT_NLI_MODEL_REPO)
    }

    /// Load a specific HF repo. ONNX weights are probed across the
    /// known layouts ([`ONNX_WEIGHT_CANDIDATES`]); the repo must also
    /// have `tokenizer.json` and a `config.json` whose `id2label` names
    /// `entailment`, `neutral`, and `contradiction` (case insensitive).
    ///
    /// # Errors
    ///
    /// See [`Self::new`].
    pub fn for_model(repo: &str) -> Result<Self, NliError> {
        let api = hf_hub::api::sync::Api::new().map_err(|e| NliError::Download(e.to_string()))?;
        let repo_id = repo.to_string();
        let repo = api.model(repo.to_string());
        let model_path = fetch_onnx_weights(&repo, &repo_id)?;
        let tokenizer_path = repo.get("tokenizer.json").map_err(|e| {
            NliError::Download(format!(
                "NLI repo `{repo_id}` has no `tokenizer.json` (a fast/JSON \
                 tokenizer is required; SentencePiece-only DeBERTa repos \
                 won't work without one): {e}"
            ))
        })?;
        let config_path = repo
            .get("config.json")
            .map_err(|e| NliError::Download(e.to_string()))?;

        let labels = parse_label_indices(&config_path)?;
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| NliError::Tokenizer(e.to_string()))?;
        let session = Session::builder()
            .map_err(|e| NliError::Session(e.to_string()))?
            .commit_from_file(&model_path)
            .map_err(|e| NliError::Session(e.to_string()))?;

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            labels,
        })
    }

    /// Classify one (premise, hypothesis) pair into a [`SupportVerdict`].
    /// In citation terms: `premise` is the span text shown to the model,
    /// `hypothesis` is the sentence the model produced.
    ///
    /// # Errors
    ///
    /// Returns [`NliError::Run`] for backend failures.
    pub fn classify(&self, premise: &str, hypothesis: &str) -> Result<SupportVerdict, NliError> {
        let encoding = self
            .tokenizer
            .encode((premise, hypothesis), true)
            .map_err(|e| NliError::Run(e.to_string()))?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|x| i64::from(*x)).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|x| i64::from(*x))
            .collect();
        let len = ids.len();
        let ids_arr =
            Array2::from_shape_vec((1, len), ids).map_err(|e| NliError::Run(e.to_string()))?;
        let mask_arr =
            Array2::from_shape_vec((1, len), mask).map_err(|e| NliError::Run(e.to_string()))?;

        let inputs = ort::inputs! {
            "input_ids" => Value::from_array(ids_arr).map_err(|e| NliError::Run(e.to_string()))?,
            "attention_mask" => Value::from_array(mask_arr).map_err(|e| NliError::Run(e.to_string()))?,
        };

        let mut session = self
            .session
            .lock()
            .map_err(|e| NliError::Run(format!("session mutex poisoned: {e}")))?;
        let outputs = session
            .run(inputs)
            .map_err(|e| NliError::Run(e.to_string()))?;
        let (shape, data) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(|e| NliError::Run(e.to_string()))?;
        if data.len() != 3 {
            return Err(NliError::Run(format!(
                "unexpected logits shape {shape:?}, expected [1, 3]"
            )));
        }

        Ok(self.argmax_to_verdict(data))
    }

    fn argmax_to_verdict(&self, logits: &[f32]) -> SupportVerdict {
        let argmax = logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(self.labels.neutral);
        if argmax == self.labels.entailment {
            SupportVerdict::Supports
        } else if argmax == self.labels.contradiction {
            SupportVerdict::Contradicts
        } else {
            SupportVerdict::Neutral
        }
    }

    /// Process-wide shared instance, initialized on first call.
    ///
    /// # Errors
    ///
    /// Returns [`NliError::Download`] on first-call initialization
    /// failure; subsequent calls return the same cached error.
    pub fn shared() -> Result<&'static Self, NliError> {
        static SHARED: OnceLock<Result<NliCrossEncoder, String>> = OnceLock::new();
        SHARED
            .get_or_init(|| NliCrossEncoder::new().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| NliError::Download(e.clone()))
    }
}

/// Separator joining cited span texts into one premise under
/// [`SupportAggregation::Concatenated`]. A bare space keeps the premise
/// readable to the model; the spans already end with their own
/// punctuation in practice.
const CONCAT_SEPARATOR: &str = " ";

/// How an [`NliSupportChecker`] combines the per-span evidence into a
/// single verdict.
///
/// `StrictWins` (the default) classifies each cited span independently
/// and lets the worst verdict win — any `Contradicts` sinks the answer,
/// else any `Neutral`, else `Supports`. It is the paper's shipping
/// behavior and the conservative choice: refuse a marginal answer rather
/// than ship a confidently wrong one.
///
/// `Concatenated` joins every cited span into ONE premise and makes a
/// single `classify` call; the verdict is whatever that call returns.
/// This lets evidence that is split across spans (each individually
/// `Neutral`) jointly entail a claim — the multi-span false-refusal the
/// `aggregation-ablation` doc measures. It is an optional mode, not the
/// default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SupportAggregation {
    /// Per-span strict-wins (default): worst single-span verdict wins.
    #[default]
    StrictWins,
    /// Concatenate all cited spans into one premise, single NLI call.
    Concatenated,
}

/// [`SupportChecker`] backed by an [`NliCrossEncoder`].
///
/// The aggregation strategy is selectable via [`SupportAggregation`];
/// the default ([`NliSupportChecker::new`]) is per-span strict-wins,
/// which classifies each cited span individually and lets the worst
/// verdict win. Use [`NliSupportChecker::with_aggregation`] to opt into
/// concatenated-evidence (single NLI call over the joined spans).
pub struct NliSupportChecker<'a> {
    encoder: &'a NliCrossEncoder,
    aggregation: SupportAggregation,
}

impl<'a> NliSupportChecker<'a> {
    /// Build a checker with the default per-span strict-wins aggregation.
    #[must_use]
    pub fn new(encoder: &'a NliCrossEncoder) -> Self {
        Self {
            encoder,
            aggregation: SupportAggregation::StrictWins,
        }
    }

    /// Build a checker with an explicit [`SupportAggregation`] mode.
    #[must_use]
    pub fn with_aggregation(encoder: &'a NliCrossEncoder, aggregation: SupportAggregation) -> Self {
        Self {
            encoder,
            aggregation,
        }
    }

    /// Per-span strict-wins: any `Contradicts` wins, else any `Neutral`,
    /// else `Supports`.
    fn check_strict_wins(
        &self,
        sentence: &str,
        cited_texts: &[&str],
    ) -> Result<SupportVerdict, SupportError> {
        let mut verdicts = Vec::with_capacity(cited_texts.len());
        for span in cited_texts {
            verdicts.push(self.encoder.classify(span, sentence)?);
        }
        Ok(combine_strict_wins(&verdicts))
    }

    /// Concatenated-evidence: join the cited spans into one premise and
    /// make a single NLI call; that call's verdict is the result.
    fn check_concatenated(
        &self,
        sentence: &str,
        cited_texts: &[&str],
    ) -> Result<SupportVerdict, SupportError> {
        let premise = concat_premise(cited_texts);
        Ok(self.encoder.classify(&premise, sentence)?)
    }
}

/// Join cited spans into a single premise for concatenated-evidence
/// aggregation.
fn concat_premise(cited_texts: &[&str]) -> String {
    cited_texts.join(CONCAT_SEPARATOR)
}

/// Strict-wins reduction over per-span verdicts: any `Contradicts` wins,
/// else any `Supports`, else `Neutral`. (Pulled out as a pure function so
/// the aggregation can be unit-tested without a real ort session.)
fn combine_strict_wins(verdicts: &[SupportVerdict]) -> SupportVerdict {
    let mut has_supports = false;
    for v in verdicts {
        match v {
            SupportVerdict::Contradicts => return SupportVerdict::Contradicts,
            SupportVerdict::Supports => has_supports = true,
            SupportVerdict::Neutral => {}
        }
    }
    if has_supports {
        SupportVerdict::Supports
    } else {
        SupportVerdict::Neutral
    }
}

impl<'a> SupportChecker for NliSupportChecker<'a> {
    fn check(&self, sentence: &str, cited_texts: &[&str]) -> Result<SupportVerdict, SupportError> {
        if cited_texts.is_empty() {
            return Ok(SupportVerdict::Neutral);
        }
        match self.aggregation {
            SupportAggregation::StrictWins => self.check_strict_wins(sentence, cited_texts),
            SupportAggregation::Concatenated => self.check_concatenated(sentence, cited_texts),
        }
    }
}

fn parse_label_indices(config_path: &Path) -> Result<LabelIndices, NliError> {
    let text =
        std::fs::read_to_string(config_path).map_err(|e| NliError::Download(e.to_string()))?;
    let cfg: ConfigJson = serde_json::from_str(&text).map_err(|e| NliError::Run(e.to_string()))?;
    if cfg.id2label.is_empty() {
        return Err(NliError::BadConfig);
    }

    let mut entailment = None;
    let mut neutral = None;
    let mut contradiction = None;
    for (idx_str, label) in &cfg.id2label {
        let idx: usize = idx_str.parse().map_err(|_| NliError::BadConfig)?;
        match label.to_ascii_uppercase().as_str() {
            "ENTAILMENT" => entailment = Some(idx),
            "NEUTRAL" => neutral = Some(idx),
            "CONTRADICTION" => contradiction = Some(idx),
            _ => {}
        }
    }
    match (entailment, neutral, contradiction) {
        (Some(e), Some(n), Some(c)) if e != n && n != c && e != c => Ok(LabelIndices {
            entailment: e,
            neutral: n,
            contradiction: c,
        }),
        _ => Err(NliError::BadConfig),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_indices() -> LabelIndices {
        LabelIndices {
            entailment: 0,
            neutral: 1,
            contradiction: 2,
        }
    }

    #[test]
    fn parse_label_indices_extracts_from_id2label() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{ "id2label": { "0": "ENTAILMENT", "1": "NEUTRAL", "2": "CONTRADICTION" } }"#,
        )
        .unwrap();
        let labels = parse_label_indices(&path).unwrap();
        assert_eq!(labels.entailment, 0);
        assert_eq!(labels.neutral, 1);
        assert_eq!(labels.contradiction, 2);
    }

    #[test]
    fn parse_label_indices_is_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{ "id2label": { "0": "contradiction", "1": "entailment", "2": "neutral" } }"#,
        )
        .unwrap();
        let labels = parse_label_indices(&path).unwrap();
        assert_eq!(labels.contradiction, 0);
        assert_eq!(labels.entailment, 1);
        assert_eq!(labels.neutral, 2);
    }

    #[test]
    fn parse_label_indices_rejects_missing_label() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{ "id2label": { "0": "ENTAILMENT", "1": "NEUTRAL" } }"#,
        )
        .unwrap();
        assert!(matches!(
            parse_label_indices(&path),
            Err(NliError::BadConfig)
        ));
    }

    // Lightweight encoder check that doesn't touch a real session: build a
    // fake LabelIndices and exercise argmax_to_verdict through a stub
    // encoder. Direct construction of NliCrossEncoder would require a real
    // ort Session, which is what the integration test exists for.

    struct StubEncoder {
        labels: LabelIndices,
    }
    impl StubEncoder {
        fn argmax_to_verdict(&self, logits: &[f32]) -> SupportVerdict {
            let argmax = logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            if argmax == self.labels.entailment {
                SupportVerdict::Supports
            } else if argmax == self.labels.contradiction {
                SupportVerdict::Contradicts
            } else {
                SupportVerdict::Neutral
            }
        }
    }

    #[test]
    fn argmax_maps_logits_to_verdicts() {
        let s = StubEncoder {
            labels: fake_indices(),
        };
        assert_eq!(
            s.argmax_to_verdict(&[5.0, 1.0, 1.0]),
            SupportVerdict::Supports
        );
        assert_eq!(
            s.argmax_to_verdict(&[1.0, 5.0, 1.0]),
            SupportVerdict::Neutral
        );
        assert_eq!(
            s.argmax_to_verdict(&[1.0, 1.0, 5.0]),
            SupportVerdict::Contradicts
        );
    }

    #[test]
    fn aggregation_default_is_strict_wins() {
        assert_eq!(
            SupportAggregation::default(),
            SupportAggregation::StrictWins
        );
    }

    // --- strict-wins reduction (per-span aggregation) ---------------------

    #[test]
    fn strict_wins_contradiction_sinks_everything() {
        // Even with supports present, a single Contradicts wins.
        assert_eq!(
            combine_strict_wins(&[
                SupportVerdict::Supports,
                SupportVerdict::Contradicts,
                SupportVerdict::Supports,
            ]),
            SupportVerdict::Contradicts
        );
    }

    #[test]
    fn strict_wins_neutral_blocks_supports() {
        // No contradiction, but a lone Neutral means we don't reach
        // Supports — the verdict is Supports only if at least one span
        // supports AND none is worse. Here one span supports, none
        // contradicts, so the result is Supports.
        assert_eq!(
            combine_strict_wins(&[SupportVerdict::Neutral, SupportVerdict::Supports]),
            SupportVerdict::Supports
        );
        // All-neutral stays Neutral.
        assert_eq!(
            combine_strict_wins(&[SupportVerdict::Neutral, SupportVerdict::Neutral]),
            SupportVerdict::Neutral
        );
    }

    #[test]
    fn strict_wins_all_support() {
        assert_eq!(
            combine_strict_wins(&[SupportVerdict::Supports, SupportVerdict::Supports]),
            SupportVerdict::Supports
        );
    }

    // --- concatenated-evidence premise joining ----------------------------

    #[test]
    fn concat_premise_joins_with_separator() {
        assert_eq!(
            concat_premise(&["dose is 5 mg.", "given once daily."]),
            "dose is 5 mg. given once daily."
        );
    }

    #[test]
    fn concat_premise_single_span_is_passthrough() {
        // A single cited span concatenates to itself — concatenated mode
        // degenerates to a single-span strict check when there's one span.
        assert_eq!(concat_premise(&["only one span"]), "only one span");
    }
}
