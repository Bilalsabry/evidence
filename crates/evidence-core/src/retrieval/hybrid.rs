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

use super::{
    bm25_search, rerank::RerankError, vector::vector_search, ChunkHit, Embedder, Reranker,
    RetrievalError,
};

/// Candidate-set multiplier for the reranked path. Asking each retriever
/// for `k * RERANK_CANDIDATE_FACTOR` candidates gives the reranker enough
/// signal to reorder meaningfully without paying for cross-encoding the
/// whole corpus.
pub const RERANK_CANDIDATE_FACTOR: usize = 4;

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

    let trimmed = query.trim();
    let sanitized = sanitize_for_fts(trimmed);
    let bm25_hits = if sanitized.is_empty() {
        Vec::new()
    } else {
        bm25_search(conn, &sanitized, candidates)?
    };

    let vec_hits = if trimmed.is_empty() {
        Vec::new()
    } else {
        let query_vec = embedder.embed(trimmed).map_err(HybridError::Embed)?;
        vector_search(conn, &query_vec, candidates)?
    };

    let fused = rrf(&bm25_hits, &vec_hits, DEFAULT_K_RRF);
    Ok(fused.into_iter().take(k).collect())
}

/// Project natural-language input onto a safe FTS5 OR-of-quoted-tokens
/// query. Drops everything that isn't alphanumeric / `-` / `_`, then quotes
/// each surviving token so FTS5 metacharacters (`?`, `*`, `-`, `:`, …)
/// can't sneak in. Returns an empty string if nothing survives.
#[must_use]
pub fn sanitize_for_fts(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 4);
    let mut first = true;
    for raw in query.split_whitespace() {
        let token: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if token.is_empty() {
            continue;
        }
        if !first {
            out.push_str(" OR ");
        }
        out.push('"');
        out.push_str(&token);
        out.push('"');
        first = false;
    }
    out
}

/// Errors that surface from hybrid retrieval.
#[derive(Debug, thiserror::Error)]
pub enum HybridError {
    #[error(transparent)]
    Retrieval(#[from] RetrievalError),
    #[error(transparent)]
    Embed(super::EmbedError),
    #[error(transparent)]
    Rerank(#[from] RerankError),
}

/// Like [`hybrid_search`], but pipes the fused candidate set through a
/// cross-encoder reranker before truncation. Asks BM25 + vector for
/// `k * RERANK_CANDIDATE_FACTOR` candidates each, fuses, reranks, then
/// truncates to `k`. The returned [`ChunkHit::score`] is the reranker
/// score (sign convention: larger is better, matching every other
/// retriever here).
///
/// # Errors
///
/// Propagates [`HybridError`] from BM25, vector search, the embedder, or
/// the reranker.
pub fn hybrid_search_with_reranker(
    conn: &Connection,
    embedder: &dyn Embedder,
    reranker: &dyn Reranker,
    query: &str,
    k: usize,
) -> Result<Vec<ChunkHit>, HybridError> {
    if k == 0 {
        return Ok(Vec::new());
    }
    let candidates = k.saturating_mul(RERANK_CANDIDATE_FACTOR).max(k);

    // Reuse hybrid_search to collect candidates; that already handles
    // empty-query short-circuit and FTS sanitization.
    let fused = hybrid_search(conn, embedder, query, candidates)?;
    if fused.is_empty() {
        return Ok(Vec::new());
    }

    let texts: Vec<String> = fused
        .iter()
        .map(|hit| chunk_text(conn, hit.chunk_id))
        .collect::<Result<_, _>>()
        .map_err(RetrievalError::from)?;
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();

    let rerank_hits = reranker.rerank(query.trim(), &refs)?;

    // rerank_hits is sorted by descending score. Walk it and rebuild a
    // ChunkHit list, swapping in the reranker score.
    let mut out: Vec<ChunkHit> = rerank_hits
        .into_iter()
        .filter_map(|r| {
            fused.get(r.index).map(|orig| ChunkHit {
                score: f64::from(r.score),
                ..orig.clone()
            })
        })
        .collect();
    out.truncate(k);
    Ok(out)
}

fn chunk_text(conn: &Connection, chunk_id: i64) -> Result<String, rusqlite::Error> {
    conn.query_row("SELECT text FROM chunks WHERE id = ?", [chunk_id], |r| {
        r.get(0)
    })
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

    #[test]
    fn sanitize_strips_fts_metacharacters() {
        let q = sanitize_for_fts("what is evidence?");
        assert_eq!(q, "\"what\" OR \"is\" OR \"evidence\"");
    }

    #[test]
    fn sanitize_keeps_alphanumeric_and_underscore() {
        let q = sanitize_for_fts("pembrolizumab_v2 trial");
        assert_eq!(q, "\"pembrolizumab_v2\" OR \"trial\"");
    }

    #[test]
    fn sanitize_empty_when_nothing_survives() {
        assert!(sanitize_for_fts("---???***").is_empty());
        assert!(sanitize_for_fts("").is_empty());
        assert!(sanitize_for_fts("   ").is_empty());
    }
}
