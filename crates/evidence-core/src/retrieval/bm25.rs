//! BM25 search over the `chunks_fts` virtual table.
//!
//! FTS5's built-in `bm25()` function returns *negative* values where smaller
//! means more relevant. [`bm25_search`] inverts the sign so larger scores
//! always mean "better hit" — that matches the contract advertised by
//! [`ChunkHit`] and keeps the future hybrid combiner ergonomic.
//!
//! FTS5 MATCH syntax (`AND`, `OR`, `NEAR`, prefix `*`, column filters)
//! passes through to the SQLite engine unchanged. Callers must escape user
//! input that contains FTS5 metacharacters; [`bm25_search`] does no
//! sanitization beyond trimming whitespace and short-circuiting empty
//! queries.

use rusqlite::{params, Connection};

use super::{ChunkHit, RetrievalError};

const SEARCH_SQL: &str = "\
    SELECT chunks.id, chunks.span_id_start, chunks.span_id_end, \
           -bm25(chunks_fts) AS score \
    FROM chunks_fts \
    JOIN chunks ON chunks.id = chunks_fts.rowid \
    WHERE chunks_fts MATCH ?1 \
    ORDER BY score DESC \
    LIMIT ?2";

/// Run a BM25 query over `chunks_fts` and return at most `k` hits, ordered
/// by descending relevance. An empty or whitespace-only `query` returns an
/// empty `Vec` without touching the database.
///
/// # Errors
///
/// Returns [`RetrievalError::Sqlite`] for SQLite or FTS5 syntax errors
/// (e.g., unbalanced quotes in `query`).
pub fn bm25_search(
    conn: &Connection,
    query: &str,
    k: usize,
) -> Result<Vec<ChunkHit>, RetrievalError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let limit = i64::try_from(k).unwrap_or(i64::MAX);
    let mut stmt = conn.prepare_cached(SEARCH_SQL)?;
    let rows = stmt
        .query_map(params![trimmed, limit], |row| {
            Ok(ChunkHit {
                chunk_id: row.get(0)?,
                span_id_start: row.get(1)?,
                span_id_end: row.get(2)?,
                score: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
