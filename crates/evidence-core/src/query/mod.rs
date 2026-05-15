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

pub mod testing;

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
    #[error("model produced a sentence with no citation: {sentence}")]
    Uncited { sentence: String },
    #[error("model cited span {span_id}, which was not in the chunks shown")]
    OutOfContext { span_id: i64 },
    #[error("model cited span {span_id}, which is not in the database")]
    UnknownSpan { span_id: i64 },
}

/// Full query pipeline: retrieve → prompt LLM → validate citations →
/// resolve citation metadata.
///
/// # Errors
///
/// Returns [`QueryError::Uncited`] / [`QueryError::OutOfContext`] /
/// [`QueryError::UnknownSpan`] when the model's output fails validation;
/// these are the "refusal" path advertised in the design doc.
pub fn answer_query(
    storage: &Storage,
    embedder: &dyn Embedder,
    llm: &dyn LlmBackend,
    question: &str,
    k: usize,
) -> Result<Answer, QueryError> {
    let hits = hybrid_search(storage.conn(), embedder, question, k)?;

    let mut prompt_chunks = Vec::with_capacity(hits.len());
    let mut allowed_spans: HashMap<i64, ()> = HashMap::new();
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
        for sid in hit.span_id_start..=hit.span_id_end {
            allowed_spans.insert(sid, ());
        }
    }

    let prompt = Prompt {
        question: question.to_string(),
        chunks: prompt_chunks,
    };

    let raw = llm.answer(&prompt)?;
    validate_and_resolve(storage, &raw, &allowed_spans)
}

fn validate_and_resolve(
    storage: &Storage,
    raw: &RawAnswer,
    allowed_spans: &HashMap<i64, ()>,
) -> Result<Answer, QueryError> {
    let mut out = Vec::with_capacity(raw.sentences.len());
    for sentence in &raw.sentences {
        if sentence.span_ids.is_empty() {
            return Err(QueryError::Uncited {
                sentence: sentence.text.clone(),
            });
        }
        let mut citations = Vec::with_capacity(sentence.span_ids.len());
        for span_id in &sentence.span_ids {
            if !allowed_spans.contains_key(span_id) {
                return Err(QueryError::OutOfContext { span_id: *span_id });
            }
            citations.push(resolve_citation(storage, *span_id)?);
        }
        out.push(Sentence {
            text: sentence.text.clone(),
            citations,
        });
    }
    Ok(Answer { sentences: out })
}

fn resolve_citation(storage: &Storage, span_id: i64) -> Result<Citation, QueryError> {
    let row = storage.conn().query_row(
        "SELECT spans.start_offset, spans.end_offset, \
                pages.page_num, pages.doc_id \
         FROM spans \
         JOIN pages ON pages.id = spans.page_id \
         WHERE spans.id = ?",
        params![span_id],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        },
    );
    match row {
        Ok((start, end, page_num, doc_id)) => Ok(Citation {
            span_id,
            doc_id,
            page_num: u32::try_from(page_num).unwrap_or(0),
            start_offset: usize::try_from(start).unwrap_or(0),
            end_offset: usize::try_from(end).unwrap_or(0),
        }),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(QueryError::UnknownSpan { span_id }),
        Err(other) => Err(QueryError::Sqlite(other)),
    }
}
