//! Hybrid retrieval: fuse BM25 and vector results with Reciprocal Rank
//! Fusion (RRF).
//!
//! RRF only needs the *rank position* in each ranked list — it ignores the
//! retrievers' raw scores, which is what makes it robust when those scores
//! live on incompatible scales (BM25 magnitudes vs. negated L2 distance).
//! Reference: Cormack, Clarke, Buettcher (2009), "Reciprocal Rank Fusion
//! outperforms Condorcet and individual Rank Learning Methods".

use std::collections::HashMap;

use rusqlite::Connection;

use super::{bm25_search, vector::vector_search, ChunkHit, Embedder, RetrievalError};

/// Default RRF constant. 60 is the value from the original paper and the de
/// facto industry default.
pub const DEFAULT_K_RRF: f64 = 60.0;

/// Fuse two ranked lists using RRF.
///
/// Each input is assumed to be ordered by descending relevance (i.e., in the
/// order [`bm25_search`] / [`vector_search`] return). `k_rrf` is the
/// smoothing constant; larger values weight tail hits more heavily.
///
/// Output is sorted by descending fused score and contains every chunk that
/// appeared in either input, exactly once.
#[must_use]
pub fn rrf(left: &[ChunkHit], right: &[ChunkHit], k_rrf: f64) -> Vec<ChunkHit> {
    let mut acc: HashMap<i64, Accum> = HashMap::with_capacity(left.len() + right.len());
    accumulate(&mut acc, left, k_rrf);
    accumulate(&mut acc, right, k_rrf);

    let mut out: Vec<ChunkHit> = acc
        .into_values()
        .map(|a| ChunkHit {
            chunk_id: a.chunk_id,
            span_id_start: a.span_id_start,
            span_id_end: a.span_id_end,
            score: a.score,
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.chunk_id.cmp(&b.chunk_id))
    });
    out
}

/// Run both retrievers and fuse their results. Asks each retriever for
/// `k * 2` candidates so RRF has enough overlap to do useful work, then
/// truncates to `k`.
///
/// # Errors
///
/// Propagates [`HybridError::Retrieval`] from BM25 or vector search and
/// [`HybridError::Embed`] from the embedder.
pub fn hybrid_search(
    conn: &Connection,
    embedder: &dyn Embedder,
    query: &str,
    k: usize,
) -> Result<Vec<ChunkHit>, HybridError> {
    let candidates = k.saturating_mul(2).max(k);

    let bm25_hits = bm25_search(conn, query, candidates)?;

    let trimmed = query.trim();
    let vec_hits = if trimmed.is_empty() {
        Vec::new()
    } else {
        let query_vec = embedder.embed(trimmed).map_err(HybridError::Embed)?;
        vector_search(conn, &query_vec, candidates)?
    };

    let fused = rrf(&bm25_hits, &vec_hits, DEFAULT_K_RRF);
    Ok(fused.into_iter().take(k).collect())
}

/// Errors that surface from hybrid retrieval.
#[derive(Debug, thiserror::Error)]
pub enum HybridError {
    #[error(transparent)]
    Retrieval(#[from] RetrievalError),
    #[error(transparent)]
    Embed(super::EmbedError),
}

struct Accum {
    chunk_id: i64,
    span_id_start: i64,
    span_id_end: i64,
    score: f64,
}

fn accumulate(acc: &mut HashMap<i64, Accum>, hits: &[ChunkHit], k_rrf: f64) {
    for (rank, hit) in hits.iter().enumerate() {
        let contribution = 1.0 / (k_rrf + (rank as f64) + 1.0);
        acc.entry(hit.chunk_id)
            .and_modify(|a| a.score += contribution)
            .or_insert_with(|| Accum {
                chunk_id: hit.chunk_id,
                span_id_start: hit.span_id_start,
                span_id_end: hit.span_id_end,
                score: contribution,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: i64) -> ChunkHit {
        ChunkHit {
            chunk_id: id,
            span_id_start: id,
            span_id_end: id,
            score: 0.0,
        }
    }

    #[test]
    fn rrf_promotes_hits_that_appear_in_both_lists() {
        // Chunk 1 is rank-1 in BM25, rank-3 in vector.
        // Chunk 2 is rank-2 in BM25 only.
        // Chunk 3 is rank-1 in vector only.
        let bm = [hit(1), hit(2)];
        let vc = [hit(3), hit(9), hit(1)];
        let merged = rrf(&bm, &vc, DEFAULT_K_RRF);
        let ranks: Vec<i64> = merged.iter().map(|h| h.chunk_id).collect();
        assert_eq!(ranks[0], 1, "both-lists chunk should top the fusion");
        assert_eq!(
            merged.iter().filter(|h| h.chunk_id == 1).count(),
            1,
            "chunk 1 must appear once in the fused output",
        );
        assert!(merged.iter().any(|h| h.chunk_id == 2));
        assert!(merged.iter().any(|h| h.chunk_id == 3));
    }

    #[test]
    fn rrf_is_stable_for_ties() {
        let bm = [hit(5), hit(2), hit(9)];
        let merged = rrf(&bm, &bm, DEFAULT_K_RRF);
        let scores: Vec<f64> = merged.iter().map(|h| h.score).collect();
        for w in scores.windows(2) {
            assert!(w[0] >= w[1]);
        }
    }

    #[test]
    fn rrf_returns_empty_for_empty_inputs() {
        assert!(rrf(&[], &[], DEFAULT_K_RRF).is_empty());
    }
}
