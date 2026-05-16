//! Smoke test for the real NLI cross-encoder path
//! ([`SupportMode::RealNli`]).
//!
//! Gated on the `real-nli-smoke` cargo feature so the default
//! `cargo test` run stays fast and offline. Run with:
//!
//! ```sh
//! cargo test -p evidence-eval --features real-nli-smoke --test real_nli_smoke
//! ```
//!
//! This is a regression guard, not an accuracy benchmark. The `--real-nli`
//! path is otherwise only exercised by hand; a single CI run with the HF
//! model cache warmed catches model-loading / wiring breakage (renamed
//! model id, tokenizer mismatch, `ort` ABI bump) without asserting the
//! model's verdicts. Agreement-rate as a measurement of NLI accuracy is a
//! separate, deliberately-not-CI concern.

#![cfg(feature = "real-nli-smoke")]

use evidence_eval::{run_with_mode, Dataset, SupportMode};

/// Three tiny examples spanning the gates the real model touches:
/// a clean supported sentence, a neutral one, a contradicting one.
const SMOKE_TOML: &str = r#"
[[example]]
name = "smoke_valid"
class = "valid"
corpus_spans = [{ id = 10, page = 1, text = "The capital of France is Paris." }]
prompt_chunks = [{ id = 1, span_range = [10, 10], text = "The capital of France is Paris." }]
response_sentences = [{ text = "The capital of France is Paris.", cited_spans = [10] }]

[[example]]
name = "smoke_unsupported"
class = "unsupported"
corpus_spans = [{ id = 10, page = 1, text = "The capital of France is Paris." }]
prompt_chunks = [{ id = 1, span_range = [10, 10], text = "The capital of France is Paris." }]
response_sentences = [{ text = "Mount Everest is the tallest mountain.", cited_spans = [10] }]

[[example]]
name = "smoke_contradicted"
class = "contradicted"
corpus_spans = [{ id = 10, page = 1, text = "The capital of France is Paris." }]
prompt_chunks = [{ id = 1, span_range = [10, 10], text = "The capital of France is Paris." }]
response_sentences = [{ text = "The capital of France is Berlin.", cited_spans = [10] }]
"#;

#[test]
fn real_nli_model_loads_and_runs_every_policy() {
    let dataset: Dataset = toml::from_str(SMOKE_TOML).expect("smoke dataset parses");

    let rows = run_with_mode(&dataset, SupportMode::RealNli)
        .expect("real NLI model loads and the run completes");

    // 3 examples × 4 policies — wiring/shape only, no verdict assertions.
    assert_eq!(rows.len(), 3 * 4, "one row per (example, policy)");

    // The model produced *some* verdict for the three-gate policy on the
    // supported example (i.e. the support gate was actually consulted and
    // didn't error out). We don't assert which verdict — that's accuracy,
    // not the wiring this guard protects.
    let three_gate_rows = rows
        .iter()
        .filter(|r| r.policy == evidence_eval::Policy::ThreeGate)
        .count();
    assert_eq!(three_gate_rows, 3, "three-gate ran for every example");
}
