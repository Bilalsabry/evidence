//! Cross-encoder rerankers.
//!
//! A reranker scores `(query, document)` pairs jointly, which produces
//! sharper relevance signals than retrieving with embeddings alone. The
//! intended flow:
//!
//! 1. [`crate::retrieval::hybrid_search`] returns a fused candidate set of
//!    size `k * 4`.
//! 2. The reranker rescores those candidates with the cross-encoder.
//! 3. The output is truncated to `k`.
//!
//! Today's shipping implementation is [`BgeReranker`], wrapping
//! `bge-reranker-base` via `fastembed`. The model auto-downloads on first
//! construction; subsequent calls reuse the cache.

use std::sync::Mutex;

use thiserror::Error;

/// Errors surfaced by a [`Reranker`].
#[derive(Debug, Error)]
pub enum RerankError {
    #[error("reranker backend failed to initialize: {0}")]
    Init(String),
    #[error("rerank call failed: {0}")]
    Run(String),
}

/// One reranker output. `index` is the position of this document in the
/// original `documents` slice passed to [`Reranker::rerank`]; `score` is
/// the cross-encoder's logit (sign convention: larger means more
/// relevant — same as [`crate::retrieval::ChunkHit::score`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RerankHit {
    pub index: usize,
    pub score: f32,
}

/// Rerank a candidate set against a query. Implementations may parallelize
/// internally and may run on CPU or GPU.
pub trait Reranker: Send + Sync {
    /// Rerank `documents` against `query`. Returned hits are sorted by
    /// descending score. Length equals `documents.len()`.
    ///
    /// # Errors
    ///
    /// Returns [`RerankError::Run`] for backend-side failures.
    fn rerank(&self, query: &str, documents: &[&str]) -> Result<Vec<RerankHit>, RerankError>;
}

/// `bge-reranker-base` via `fastembed`. The model and the ONNX runtime are
/// downloaded into the HF cache on first construction.
///
/// `fastembed::TextRerank::rerank` takes `&mut self`, so the inner model
/// lives behind a [`Mutex`] — same pattern as
/// [`crate::retrieval::BgeSmall`].
pub struct BgeReranker {
    inner: Mutex<fastembed::TextRerank>,
}

impl BgeReranker {
    /// Initialize the reranker. First call downloads the model
    /// (~280 MB); subsequent calls reuse the cache.
    ///
    /// # Errors
    ///
    /// Returns [`RerankError::Init`] if the model download fails or the
    /// ONNX runtime cannot start.
    pub fn new() -> Result<Self, RerankError> {
        use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
        let inner = TextRerank::try_new(RerankInitOptions::new(RerankerModel::BGERerankerBase))
            .map_err(|e| RerankError::Init(e.to_string()))?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }
}

impl Reranker for BgeReranker {
    fn rerank(&self, query: &str, documents: &[&str]) -> Result<Vec<RerankHit>, RerankError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| RerankError::Run(format!("reranker mutex poisoned: {e}")))?;
        let results = guard
            .rerank(query, documents, false, None)
            .map_err(|e| RerankError::Run(e.to_string()))?;
        Ok(results
            .into_iter()
            .map(|r| RerankHit {
                index: r.index,
                score: r.score,
            })
            .collect())
    }
}
