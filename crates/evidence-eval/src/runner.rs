//! Runner: applies each [`Policy`] to every example in a dataset and
//! records what the validator did.
//!
//! The harness uses an in-memory SQLite store per example: seed the
//! corpus, build a [`Prompt`] from the example's `prompt_chunks`, build
//! a [`RawAnswer`] from the example's `response_sentences`, and call
//! [`evidence_core::query::validate_answer`].

use std::collections::HashMap;

use evidence_core::query::{
    validate_answer, ChunkContext, NliCrossEncoder, NliSupportChecker, Prompt, QueryError,
    RawAnswer, RawSentence, SupportAggregation, SupportChecker, SupportError, SupportVerdict,
    ValidationPolicy,
};
use evidence_core::storage::Storage;
use rusqlite::params;

use crate::dataset::{Dataset, Example, HallucinationClass};

/// How the harness materializes the support gate's checker.
///
/// `Mock` substitutes a class-derived mock so the runner is
/// deterministic — the harness tests assert validator wiring under this
/// mode. `RealNli` loads `NliCrossEncoder::shared()` and lets the actual
/// model decide; under this mode, "agreement" with the expected matrix
/// becomes a measurement of the NLI model's accuracy on our labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportMode {
    Mock,
    RealNli,
}

/// Validator configurations the harness evaluates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Policy {
    /// No validation at all.
    VanillaRag,
    /// Require citations + existence.
    ExistenceOnly,
    /// Existence + in-context (the closed-loop two-gate).
    TwoGate,
    /// Existence + in-context + support (the closed-loop three-gate).
    /// The harness uses a class-driven mock checker so the test stays
    /// deterministic; a real-model run swaps the mock for
    /// `NliSupportChecker`.
    ThreeGate,
}

impl Policy {
    /// Stable name for reports.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Policy::VanillaRag => "vanilla_rag",
            Policy::ExistenceOnly => "existence_only",
            Policy::TwoGate => "two_gate",
            Policy::ThreeGate => "three_gate",
        }
    }

    /// All policies in stable order. The report ordering matches.
    #[must_use]
    pub fn all() -> [Policy; 4] {
        [
            Policy::VanillaRag,
            Policy::ExistenceOnly,
            Policy::TwoGate,
            Policy::ThreeGate,
        ]
    }
}

/// What the validator did with one (example, policy) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Validator accepted the answer.
    Accepted,
    /// Validator refused. The variant captures *which* gate refused.
    Refused(RefusalReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// Sentence had no citations.
    Uncited,
    /// Cited span didn't exist in the DB.
    UnknownSpan,
    /// Cited span wasn't in the prompt's chunks.
    OutOfContext,
    /// Cited span didn't entail the sentence.
    Unsupported,
    /// Cited span contradicted the sentence.
    Contradicted,
    /// Anything else (mostly SQLite errors during eval setup — fail loud).
    Other(String),
}

/// One row of the report.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub example: String,
    pub class: HallucinationClass,
    pub policy: Policy,
    pub outcome: Outcome,
    pub expected: Outcome,
    pub agreement: bool,
}

/// Run every example against every policy with class-derived mocks for
/// the support gate. The agreement rate is always 100% by construction;
/// what the harness measures here is *validator wiring*, not model
/// behavior.
///
/// # Errors
///
/// Returns an error if dataset seeding fails.
pub fn run(dataset: &Dataset) -> anyhow::Result<Vec<RunResult>> {
    run_with_mode(dataset, SupportMode::Mock)
}

/// Run every example against every policy, choosing how the support gate
/// gets its verdicts via [`SupportMode`].
///
/// Under [`SupportMode::RealNli`], the support gate uses
/// [`NliCrossEncoder::shared()`] — the same process-wide instance the CLI
/// uses. The agreement rate then reports the NLI model's accuracy on the
/// labeled examples: a row disagrees with the expected matrix exactly
/// when the model's verdict differs from the label.
///
/// # Errors
///
/// Returns an error if dataset seeding fails or — under
/// [`SupportMode::RealNli`] — the NLI model fails to initialize.
pub fn run_with_mode(dataset: &Dataset, mode: SupportMode) -> anyhow::Result<Vec<RunResult>> {
    run_with_model(dataset, mode, None)
}

/// Like [`run_with_mode`] but, under [`SupportMode::RealNli`], lets the
/// caller substitute a different HuggingFace NLI repo for the default
/// (`distilbert-base-uncased-mnli`). Pass `Some(repo)` to measure a
/// stronger model (e.g. a DeBERTa-v3-large MNLI checkpoint) on the same
/// labeled set — the §5.1 model-comparison the paper needs. `None`
/// reuses the process-wide shared default (cached across calls).
///
/// # Errors
///
/// Returns an error if dataset seeding fails or — under
/// [`SupportMode::RealNli`] — the NLI model fails to initialize.
pub fn run_with_model(
    dataset: &Dataset,
    mode: SupportMode,
    nli_model: Option<&str>,
) -> anyhow::Result<Vec<RunResult>> {
    run_with_options(dataset, mode, nli_model, SupportAggregation::default())
}

/// Like [`run_with_model`] but also selects the support gate's
/// [`SupportAggregation`] mode used by the real NLI checker.
/// `SupportAggregation::StrictWins` (the default) reproduces
/// [`run_with_model`]; `SupportAggregation::Concatenated` joins each
/// sentence's cited spans into one premise and does a single NLI call —
/// the concatenated-evidence ablation. The aggregation only affects
/// [`SupportMode::RealNli`]; under [`SupportMode::Mock`] the class-derived
/// mock is unchanged.
///
/// # Errors
///
/// Returns an error if dataset seeding fails or — under
/// [`SupportMode::RealNli`] — the NLI model fails to initialize.
pub fn run_with_options(
    dataset: &Dataset,
    mode: SupportMode,
    nli_model: Option<&str>,
    aggregation: SupportAggregation,
) -> anyhow::Result<Vec<RunResult>> {
    // A custom repo gets an owned encoder scoped to this call; the
    // default reuses the cached process-wide instance. Either way the
    // checker borrows it for the loop below — no `'static` needed.
    let owned_encoder: Option<NliCrossEncoder> = match (mode, nli_model) {
        (SupportMode::RealNli, Some(repo)) => Some(
            NliCrossEncoder::for_model(repo)
                .map_err(|e| anyhow::anyhow!("NLI init ({repo}): {e}"))?,
        ),
        _ => None,
    };
    // For RealNli mode, load the model once before iterating so first-
    // example latency doesn't include the download.
    let nli_encoder: Option<&NliCrossEncoder> = match mode {
        SupportMode::Mock => None,
        SupportMode::RealNli => Some(match owned_encoder.as_ref() {
            Some(e) => e,
            None => NliCrossEncoder::shared().map_err(|e| anyhow::anyhow!("NLI init: {e}"))?,
        }),
    };

    let mut rows = Vec::with_capacity(dataset.examples.len() * Policy::all().len());
    for example in &dataset.examples {
        let storage = seed_storage(example)?;
        let prompt = build_prompt(example);
        let raw = build_raw_answer(example);
        for policy in Policy::all() {
            // The checker is only consulted when the policy is ThreeGate
            // *and* it's `Some`. For non-ThreeGate policies we still pass
            // it so the harness's shape stays uniform.
            let (mock_owner, nli_checker_owner) =
                build_checker(mode, nli_encoder, aggregation, example.class);
            let checker_ref: Option<&dyn SupportChecker> = mock_owner
                .as_deref()
                .or_else(|| nli_checker_owner.as_ref().map(|c| c as &dyn SupportChecker));
            let policy_obj = build_policy(policy, checker_ref);
            let outcome = match validate_answer(&storage, &raw, &prompt, &policy_obj) {
                Ok(_) => Outcome::Accepted,
                Err(err) => Outcome::Refused(classify_refusal(&err)),
            };
            let expected = expected_outcome(example.class, policy);
            let agreement = outcome == expected;
            rows.push(RunResult {
                example: example.name.clone(),
                class: example.class,
                policy,
                outcome,
                expected,
                agreement,
            });
        }
    }
    Ok(rows)
}

/// Build the support checker for one (mode, example) pair. Returns the
/// owners so the runner can take references without the checker being
/// dropped on the spot.
fn build_checker<'e>(
    mode: SupportMode,
    nli_encoder: Option<&'e NliCrossEncoder>,
    aggregation: SupportAggregation,
    class: HallucinationClass,
) -> (
    Option<Box<dyn SupportChecker>>,
    Option<NliSupportChecker<'e>>,
) {
    match mode {
        SupportMode::Mock => (mock_support_for(class), None),
        SupportMode::RealNli => {
            let encoder = nli_encoder.expect("RealNli mode loaded the encoder up front");
            (
                None,
                Some(NliSupportChecker::with_aggregation(encoder, aggregation)),
            )
        }
    }
}

fn build_policy<'a>(
    policy: Policy,
    checker: Option<&'a dyn SupportChecker>,
) -> ValidationPolicy<'a> {
    match policy {
        Policy::VanillaRag => ValidationPolicy::vanilla_rag(),
        Policy::ExistenceOnly => ValidationPolicy::existence_only(),
        Policy::TwoGate => ValidationPolicy::closed_loop_two_gate(),
        Policy::ThreeGate => {
            // If no checker supplied, fall back to two-gate behavior.
            // For class-driven mocks the checker is always Some(...) in
            // the runner's call sites below.
            checker
                .map(ValidationPolicy::closed_loop_three_gate)
                .unwrap_or_else(ValidationPolicy::closed_loop_two_gate)
        }
    }
}

fn build_prompt(example: &Example) -> Prompt {
    let chunks = example
        .prompt_chunks
        .iter()
        .map(|c| ChunkContext {
            chunk_id: c.id,
            text: c.text.clone(),
            span_id_start: c.span_range[0],
            span_id_end: c.span_range[1],
        })
        .collect();
    Prompt {
        question: format!("eval:{}", example.name),
        chunks,
    }
}

fn build_raw_answer(example: &Example) -> RawAnswer {
    let sentences = example
        .response_sentences
        .iter()
        .map(|s| RawSentence {
            text: s.text.clone(),
            span_ids: s.cited_spans.clone(),
        })
        .collect();
    RawAnswer { sentences }
}

fn seed_storage(example: &Example) -> anyhow::Result<Storage> {
    let storage = Storage::open_in_memory()?;
    let conn = storage.conn();
    conn.execute(
        "INSERT INTO documents (sha256, page_count, ingested_at) VALUES (?, ?, 0)",
        params![&example.name, example.corpus_spans.len() as i64],
    )?;
    let doc_id: i64 = conn.query_row(
        "SELECT id FROM documents WHERE sha256 = ?",
        [&example.name],
        |r| r.get(0),
    )?;

    // Group spans by page so each page is inserted once.
    let mut pages: HashMap<u32, i64> = HashMap::new();
    for s in &example.corpus_spans {
        if let std::collections::hash_map::Entry::Vacant(e) = pages.entry(s.page) {
            conn.execute(
                "INSERT INTO pages (doc_id, page_num, raw_text) VALUES (?, ?, '')",
                params![doc_id, i64::from(s.page)],
            )?;
            e.insert(conn.last_insert_rowid());
        }
    }

    for s in &example.corpus_spans {
        let page_id = pages[&s.page];
        let len = i64::try_from(s.text.len()).unwrap_or(0);
        conn.execute(
            "INSERT INTO spans (id, page_id, start_offset, end_offset, text, bbox_json) \
             VALUES (?, ?, 0, ?, ?, '{}')",
            params![s.id, page_id, len.max(1), &s.text],
        )?;
    }
    Ok(storage)
}

/// Mock [`SupportChecker`] derived from the example's class. Used by the
/// runner so the three-gate policy's behavior is determined by the
/// example's label, not by a stochastic model. The eval crate's real-model
/// runner (planned for the paper) swaps this for `NliSupportChecker`.
fn mock_support_for(class: HallucinationClass) -> Option<Box<dyn SupportChecker>> {
    match class {
        HallucinationClass::Unsupported => Some(Box::new(NeutralChecker)),
        HallucinationClass::Contradicted => Some(Box::new(ContradictingChecker)),
        // For every other class, the support check, if reached, should
        // accept — the refusal (if any) comes from an earlier gate.
        _ => Some(Box::new(SupportingChecker)),
    }
}

struct SupportingChecker;
impl SupportChecker for SupportingChecker {
    fn check(&self, _: &str, _: &[&str]) -> Result<SupportVerdict, SupportError> {
        Ok(SupportVerdict::Supports)
    }
}
struct NeutralChecker;
impl SupportChecker for NeutralChecker {
    fn check(&self, _: &str, _: &[&str]) -> Result<SupportVerdict, SupportError> {
        Ok(SupportVerdict::Neutral)
    }
}
struct ContradictingChecker;
impl SupportChecker for ContradictingChecker {
    fn check(&self, _: &str, _: &[&str]) -> Result<SupportVerdict, SupportError> {
        Ok(SupportVerdict::Contradicts)
    }
}

fn classify_refusal(err: &QueryError) -> RefusalReason {
    match err {
        QueryError::Uncited { .. } => RefusalReason::Uncited,
        QueryError::UnknownSpan { .. } => RefusalReason::UnknownSpan,
        QueryError::OutOfContext { .. } => RefusalReason::OutOfContext,
        QueryError::Unsupported { .. } => RefusalReason::Unsupported,
        QueryError::Contradicted { .. } => RefusalReason::Contradicted,
        other => RefusalReason::Other(other.to_string()),
    }
}

/// The harness's ground truth: given a class + policy, what should the
/// validator do? This is the matrix the paper claims and tests
/// reproduce.
#[must_use]
pub fn expected_outcome(class: HallucinationClass, policy: Policy) -> Outcome {
    use HallucinationClass as C;
    use Policy as P;
    use RefusalReason as R;
    match (class, policy) {
        // Valid sentences pass every policy.
        (C::Valid, _) => Outcome::Accepted,

        // Uncited: vanilla accepts (no shape requirement); every other
        // policy requires a citation and refuses.
        (C::Uncited, P::VanillaRag) => Outcome::Accepted,
        (C::Uncited, _) => Outcome::Refused(R::Uncited),

        // Fabricated span: vanilla accepts; existence gate catches.
        (C::FabricatedSpan, P::VanillaRag) => Outcome::Accepted,
        (C::FabricatedSpan, _) => Outcome::Refused(R::UnknownSpan),

        // Out-of-context: vanilla + existence accept (the span exists in
        // the DB); two-gate and three-gate refuse.
        (C::OutOfContext, P::VanillaRag | P::ExistenceOnly) => Outcome::Accepted,
        (C::OutOfContext, _) => Outcome::Refused(R::OutOfContext),

        // Unsupported: only three-gate refuses (Neutral verdict).
        (C::Unsupported, P::ThreeGate) => Outcome::Refused(R::Unsupported),
        (C::Unsupported, _) => Outcome::Accepted,

        // Contradicted: only three-gate refuses (Contradicts verdict).
        (C::Contradicted, P::ThreeGate) => Outcome::Refused(R::Contradicted),
        (C::Contradicted, _) => Outcome::Accepted,
    }
}
