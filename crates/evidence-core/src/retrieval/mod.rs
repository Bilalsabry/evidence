//! Retrieval primitives over the storage layer.
//!
//! Today: BM25 over `chunks_fts`. Issue #4 adds vector search over
//! `chunks_vec` and a reciprocal-rank-fusion combiner; both will return the
//! same [`ChunkHit`] shape so callers can swap retrievers freely.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod bm25;
pub mod embed;
pub mod hybrid;
pub mod vector;

pub use bm25::bm25_search;
pub use embed::{BgeSmall, EmbedError, Embedder, BGE_SMALL_DIM};
pub use hybrid::{hybrid_search, rrf};
pub use vector::{upsert_chunk_embedding, vector_search};

/// A single retrieval result. Identifies the chunk and the span range it
/// covers; [`score`](Self::score) is retriever-specific and comparable only
/// within a single result set.
///
/// For BM25, larger `score` is better (sign-inverted from SQLite's `bm25()`,
/// which returns negative values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkHit {
    pub chunk_id: i64,
    pub span_id_start: i64,
    pub span_id_end: i64,
    pub score: f64,
}

/// Errors surfaced by retrieval calls.
#[derive(Debug, Error)]
pub enum RetrievalError {
    /// Underlying SQLite or FTS5 error.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}
