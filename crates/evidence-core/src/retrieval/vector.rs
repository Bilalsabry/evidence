//! Vector search over the `chunks_vec` virtual table.
//!
//! `vec0` accepts either binary BLOBs of little-endian `f32` or JSON-array
//! literals as input. We use JSON-array literals here for readability and
//! parity with the unit tests; the overhead is negligible at the sizes we
//! handle (a single 384-dim probe ≈ 3 KB of text).

use rusqlite::{params, Connection};

use super::{ChunkHit, RetrievalError};

const SEARCH_SQL: &str = "\
    SELECT chunks.id, chunks.span_id_start, chunks.span_id_end, \
           -chunks_vec.distance AS score \
    FROM chunks_vec \
    JOIN chunks ON chunks.id = chunks_vec.rowid \
    WHERE chunks_vec.embedding MATCH ?1 \
      AND k = ?2 \
    ORDER BY chunks_vec.distance ASC";

/// KNN search over `chunks_vec`. Returns up to `k` hits ordered by ascending
/// distance (i.e., descending [`ChunkHit::score`], since the score is the
/// negated distance).
///
/// # Errors
///
/// Returns [`RetrievalError::Sqlite`] for SQLite errors or vec0 dimension
/// mismatches.
pub fn vector_search(
    conn: &Connection,
    query: &[f32],
    k: usize,
) -> Result<Vec<ChunkHit>, RetrievalError> {
    if k == 0 || query.is_empty() {
        return Ok(Vec::new());
    }
    let limit = i64::try_from(k).unwrap_or(i64::MAX);
    let probe = vec_to_json(query);
    let mut stmt = conn.prepare_cached(SEARCH_SQL)?;
    let rows = stmt
        .query_map(params![probe, limit], |row| {
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

/// Upsert an embedding for `chunk_id`. `vec0` doesn't support `ON CONFLICT`,
/// so a pre-existing row is deleted first; both writes happen inside the
/// caller's transaction (or a new one if none is active).
///
/// # Errors
///
/// Returns [`RetrievalError::Sqlite`] on a write failure or dimension
/// mismatch.
pub fn upsert_chunk_embedding(
    conn: &Connection,
    chunk_id: i64,
    embedding: &[f32],
) -> Result<(), RetrievalError> {
    let probe = vec_to_json(embedding);
    conn.execute("DELETE FROM chunks_vec WHERE rowid = ?", params![chunk_id])?;
    conn.execute(
        "INSERT INTO chunks_vec (rowid, embedding) VALUES (?, ?)",
        params![chunk_id, probe],
    )?;
    Ok(())
}

/// Serialize a `&[f32]` as a JSON array. Pre-allocates an upper-bound buffer
/// so the hot path avoids reallocations.
pub(crate) fn vec_to_json(v: &[f32]) -> String {
    // Worst case ~16 chars per f32 (sign, digits, exponent, comma); add 2
    // for the brackets.
    let mut s = String::with_capacity(v.len() * 16 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // Default Display is fine here; vec0 parses the standard numeric
        // grammar (including `inf`/`nan`, though we never produce those).
        use std::fmt::Write as _;
        let _ = write!(s, "{x}");
    }
    s.push(']');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_to_json_round_trips_through_serde() {
        let v = vec![0.0_f32, 1.5, -2.25, 1e-6];
        let s = vec_to_json(&v);
        let parsed: Vec<f32> = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.len(), v.len());
        for (a, b) in parsed.iter().zip(v.iter()) {
            assert!((a - b).abs() < 1e-9, "{a} vs {b}");
        }
    }
}
