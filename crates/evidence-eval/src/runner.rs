//! Runner: applies each [`Policy`] to every example in a dataset and
//! records what the validator did.
//!
//! The harness uses an in-memory SQLite store per example: seed the
//! corpus, build a [`Prompt`] from the example's `prompt_chunks`, build
//! a [`RawAnswer`] from the example's `response_sentences`, and call
//! [`evidence_core::query::validate_answer`].

use std::collections::HashMap;

use evidence_core::query::{
    validate_answer, ChunkContext, Prompt, QueryError, RawAnswer, RawSentence, SupportChecker,
    SupportError, SupportVerdict, ValidationPolicy,
};
use evidence_core::storage::Storage;
use rusqlite::params;

use crate::dataset::{Dataset, Example, HallucinationClass};

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

/// Run every example against every policy. Returns one row per
/// (example, policy) pair.
///
/// # Errors
///
/// Returns an error if dataset seeding fails (in-memory SQLite shouldn't
/// fail, but we surface anyway).
pub fn run(dataset: &Dataset) -> anyhow::Result<Vec<RunResult>> {
    let mut rows = Vec::with_capacity(dataset.examples.len() * Policy::all().len());
    for example in &dataset.examples {
        let storage = seed_storage(example)?;
        let prompt = build_prompt(example);
        let raw = build_raw_answer(example);
        for policy in Policy::all() {
            let checker = mock_support_for(example.class);
            let policy_obj = build_policy(policy, checker.as_deref());
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
