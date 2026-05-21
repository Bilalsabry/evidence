//! Question-answering pipeline with span-level citation enforcement.
//!
//! [`answer_query`] is the public entry point. It retrieves the top-`k`
//! chunks from the [hybrid retriever](crate::retrieval::hybrid_search),
//! hands them to an [`LlmBackend`] together with the question, validates
//! the citations returned by the model, and returns either an [`Answer`]
//! (every sentence resolves to spans the model was shown) or a typed error
//! (the model produced a sentence that didn't).
//!
//! The trait lives here so callers can swap in any backend: a real one in
//! the CLI today, future remote-gateway adapters, or the deterministic
//! [`testing::MockBackend`] used in tests.

use std::collections::HashMap;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::retrieval::{hybrid_search, Embedder, HybridError};
use crate::storage::Storage;

pub mod nli;
pub mod support;
pub mod testing;

pub use nli::{
    NliCrossEncoder, NliError, NliSupportChecker, SupportAggregation, DEFAULT_NLI_MODEL_REPO,
};
pub use support::{
    RerankerSupportChecker, SupportChecker, SupportError, SupportVerdict, SUPPORTED_THRESHOLD,
};

/// One context chunk shown to the LLM.
///
/// `span_id_start..=span_id_end` is the inclusive range of span IDs covered
/// by the chunk; the validator allows the model to cite any span in that
/// range.
#[derive(Debug, Clone, Serialize)]
pub struct ChunkContext {
    pub chunk_id: i64,
    pub text: String,
    pub span_id_start: i64,
    pub span_id_end: i64,
}

/// What the pipeline hands to an [`LlmBackend`].
#[derive(Debug, Clone, Serialize)]
pub struct Prompt {
    pub question: String,
    pub chunks: Vec<ChunkContext>,
}

/// Unvalidated answer shape. The backend must return this; the validator
/// turns it into an [`Answer`] (or a refusal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAnswer {
    pub sentences: Vec<RawSentence>,
}

/// One sentence as the model produced it, with raw span-id references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSentence {
    pub text: String,
    pub span_ids: Vec<i64>,
}

/// A validated answer: every sentence has at least one citation, and every
/// citation resolves to a real span the model was shown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub sentences: Vec<Sentence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sentence {
    pub text: String,
    pub citations: Vec<Citation>,
}

/// A single resolved citation. Carries enough information for a UI to deep-
/// link into the source document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub span_id: i64,
    pub doc_id: i64,
    pub page_num: u32,
    pub start_offset: usize,
    pub end_offset: usize,
}

/// Errors returned by a backend.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("LLM backend refused to answer")]
    Refused,
    #[error("LLM backend transport error: {0}")]
    Transport(String),
    #[error("LLM produced malformed output: {0}")]
    Malformed(String),
}

/// Implementors turn a [`Prompt`] into a [`RawAnswer`]. Real backends live
/// in consumer crates; in-tree tests use [`testing::MockBackend`].
pub trait LlmBackend: Send + Sync {
    /// Answer a prompt. Implementations should request structured output
    /// when the backend supports it.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError`] for transport or parsing failures.
    fn answer(&self, prompt: &Prompt) -> Result<RawAnswer, LlmError>;
}

/// Errors surfaced by [`answer_query`].
#[derive(Debug, Error)]
pub enum QueryError {
    #[error(transparent)]
    Retrieval(#[from] HybridError),
    #[error(transparent)]
    Llm(#[from] LlmError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Support(#[from] SupportError),
    #[error("model produced a sentence with no citation: {sentence}")]
    Uncited { sentence: String },
    #[error("model cited span {span_id}, which was not in the chunks shown")]
    OutOfContext { span_id: i64 },
    #[error("model cited span {span_id}, which is not in the database")]
    UnknownSpan { span_id: i64 },
    #[error("cited spans do not lexically support the sentence: {sentence}")]
    Unsupported { sentence: String },
    #[error("cited spans contradict the sentence: {sentence}")]
    Contradicted { sentence: String },
}

/// Toggle for each validator gate. Lets callers ablate the validator for
/// evaluation work (see the `evidence-eval` crate) — for example,
/// disabling [`Self::enforce_in_context`] measures how much the
/// closed-loop constraint contributes to the catch rate compared to a
/// generic existence-only validator.
///
/// Production callers should use [`Self::closed_loop_two_gate`] (v0.1
/// reference) or [`Self::closed_loop_three_gate`] (v0.3 reference, full
/// Closed-Loop Citation).
#[derive(Clone, Copy)]
pub struct ValidationPolicy<'a> {
    /// Reject sentences that carry no span citations. Disabling collapses
    /// the system to vanilla RAG: any text the model produces is
    /// accepted.
    pub require_citations: bool,
    /// Reject citations whose span ID isn't in the database. Disabling
    /// accepts fabricated span IDs.
    pub enforce_existence: bool,
    /// Reject citations whose span ID wasn't in the chunks shown to the
    /// model for this query. This is the closed-loop constraint —
    /// disabling lets the model cite anything in the corpus, even spans
    /// it didn't read.
    pub enforce_in_context: bool,
    /// If `Some`, every accepted sentence is run through an NLI-style
    /// support check.
    pub support: Option<&'a dyn SupportChecker>,
}

impl Default for ValidationPolicy<'_> {
    fn default() -> Self {
        Self::closed_loop_two_gate()
    }
}

impl<'a> ValidationPolicy<'a> {
    /// All gates off. Baseline: vanilla RAG where any model output is
    /// accepted regardless of citation shape or validity.
    #[must_use]
    pub fn vanilla_rag() -> Self {
        Self {
            require_citations: false,
            enforce_existence: false,
            enforce_in_context: false,
            support: None,
        }
    }

    /// Require citations + existence only. Catches fabricated span IDs
    /// but accepts cross-corpus citations the model never actually read.
    #[must_use]
    pub fn existence_only() -> Self {
        Self {
            require_citations: true,
            enforce_existence: true,
            enforce_in_context: false,
            support: None,
        }
    }

    /// Closed-Loop Citation, two gates: existence + in-context. The v0.1
    /// reference: the model can only cite spans it was actually shown.
    #[must_use]
    pub fn closed_loop_two_gate() -> Self {
        Self {
            require_citations: true,
            enforce_existence: true,
            enforce_in_context: true,
            support: None,
        }
    }

    /// Closed-Loop Citation, three gates: existence + in-context +
    /// entailment. The v0.3 reference. `support` is the NLI checker (or
    /// the v0.2 reranker proxy).
    #[must_use]
    pub fn closed_loop_three_gate(support: &'a dyn SupportChecker) -> Self {
        Self {
            require_citations: true,
            enforce_existence: true,
            enforce_in_context: true,
            support: Some(support),
        }
    }
}

/// Full query pipeline: retrieve → prompt LLM → validate citations →
/// resolve citation metadata.
///
/// # Errors
///
/// Returns [`QueryError::Uncited`] / [`QueryError::OutOfContext`] /
/// [`QueryError::UnknownSpan`] / [`QueryError::Unsupported`] /
/// [`QueryError::Contradicted`] when the model's output fails the
/// configured validator gates.
pub fn answer_query(
    storage: &Storage,
    embedder: &dyn Embedder,
    llm: &dyn LlmBackend,
    question: &str,
    k: usize,
    support: Option<&dyn SupportChecker>,
) -> Result<Answer, QueryError> {
    let policy = match support {
        Some(s) => ValidationPolicy::closed_loop_three_gate(s),
        None => ValidationPolicy::closed_loop_two_gate(),
    };
    answer_query_with_policy(storage, embedder, llm, question, k, &policy)
}

/// Same as [`answer_query`], but explicit about the validator's gate
/// configuration. Used by the eval harness to ablate one gate at a time.
///
/// # Errors
///
/// See [`answer_query`].
pub fn answer_query_with_policy(
    storage: &Storage,
    embedder: &dyn Embedder,
    llm: &dyn LlmBackend,
    question: &str,
    k: usize,
    policy: &ValidationPolicy<'_>,
) -> Result<Answer, QueryError> {
    let hits = hybrid_search(storage.conn(), embedder, question, k)?;

    let mut prompt_chunks = Vec::with_capacity(hits.len());
    for hit in &hits {
        let text: String = storage.conn().query_row(
            "SELECT text FROM chunks WHERE id = ?",
            [hit.chunk_id],
            |r| r.get(0),
        )?;
        prompt_chunks.push(ChunkContext {
            chunk_id: hit.chunk_id,
            text,
            span_id_start: hit.span_id_start,
            span_id_end: hit.span_id_end,
        });
    }

    let prompt = Prompt {
        question: question.to_string(),
        chunks: prompt_chunks,
    };

    let raw = llm.answer(&prompt)?;
    validate_answer(storage, &raw, &prompt, policy)
}

/// Run the validator against a pre-built `(Prompt, RawAnswer)` pair —
/// no retrieval, no LLM call. This is the surface the eval harness uses
/// to feed fixture model outputs through each [`ValidationPolicy`] and
/// measure which gates catch which hallucination class.
///
/// # Errors
///
/// See [`answer_query`].
pub fn validate_answer(
    storage: &Storage,
    raw: &RawAnswer,
    prompt: &Prompt,
    policy: &ValidationPolicy<'_>,
) -> Result<Answer, QueryError> {
    let allowed_spans: HashMap<i64, ()> = prompt
        .chunks
        .iter()
        .flat_map(|c| (c.span_id_start..=c.span_id_end).map(|s| (s, ())))
        .collect();
    validate_and_resolve(storage, raw, &allowed_spans, policy)
}

fn validate_and_resolve(
    storage: &Storage,
    raw: &RawAnswer,
    allowed_spans: &HashMap<i64, ()>,
    policy: &ValidationPolicy<'_>,
) -> Result<Answer, QueryError> {
    let mut out = Vec::with_capacity(raw.sentences.len());
    for sentence in &raw.sentences {
        // Shape gate: require at least one citation per sentence.
        if policy.require_citations && sentence.span_ids.is_empty() {
            return Err(QueryError::Uncited {
                sentence: sentence.text.clone(),
            });
        }

        let mut citations = Vec::with_capacity(sentence.span_ids.len());
        let mut cited_texts: Vec<String> = Vec::with_capacity(sentence.span_ids.len());
        for span_id in &sentence.span_ids {
            // Gate 1 (existence): the span ID must resolve in the DB.
            // We run existence before in-context so each gate cleanly
            // catches its own hallucination class — a fabricated ID
            // surfaces as `UnknownSpan` even if it also happens to be
            // outside the prompt's allowed set.
            let lookup = resolve_citation_with_text(storage, *span_id);
            let (citation, text) = match lookup {
                Ok(pair) => pair,
                Err(QueryError::UnknownSpan { .. }) if !policy.enforce_existence => (
                    Citation {
                        span_id: *span_id,
                        doc_id: 0,
                        page_num: 0,
                        start_offset: 0,
                        end_offset: 0,
                    },
                    String::new(),
                ),
                Err(e) => return Err(e),
            };

            // Gate 2 (in-context): the span must have been in the prompt.
            if policy.enforce_in_context && !allowed_spans.contains_key(span_id) {
                return Err(QueryError::OutOfContext { span_id: *span_id });
            }

            citations.push(citation);
            cited_texts.push(text);
        }

        // Gate 3 (entailment/support).
        if let Some(checker) = policy.support {
            let refs: Vec<&str> = cited_texts.iter().map(String::as_str).collect();
            match checker.check(&sentence.text, &refs)? {
                SupportVerdict::Supports => {}
                SupportVerdict::Neutral => {
                    return Err(QueryError::Unsupported {
                        sentence: sentence.text.clone(),
                    });
                }
                SupportVerdict::Contradicts => {
                    return Err(QueryError::Contradicted {
                        sentence: sentence.text.clone(),
                    });
                }
            }
        }

        out.push(Sentence {
            text: sentence.text.clone(),
            citations,
        });
    }
    Ok(Answer { sentences: out })
}

fn resolve_citation_with_text(
    storage: &Storage,
    span_id: i64,
) -> Result<(Citation, String), QueryError> {
    let row = storage.conn().query_row(
        "SELECT spans.start_offset, spans.end_offset, spans.text, \
                pages.page_num, pages.doc_id \
         FROM spans \
         JOIN pages ON pages.id = spans.page_id \
         WHERE spans.id = ?",
        params![span_id],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            ))
        },
    );
    match row {
        Ok((start, end, text, page_num, doc_id)) => Ok((
            Citation {
                span_id,
                doc_id,
                page_num: u32::try_from(page_num).unwrap_or(0),
                start_offset: usize::try_from(start).unwrap_or(0),
                end_offset: usize::try_from(end).unwrap_or(0),
            },
            text,
        )),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(QueryError::UnknownSpan { span_id }),
        Err(other) => Err(QueryError::Sqlite(other)),
    }
}
