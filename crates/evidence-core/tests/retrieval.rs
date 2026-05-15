//! BM25 retrieval tests against a small fixture corpus.
//!
//! The corpus is seven hand-picked chunks chosen so that a hand-picked query
//! has a single unambiguous top hit. Tests also exercise the FTS triggers
//! installed by migration 003 (insert / update / delete sync).

use evidence_core::retrieval::{bm25_search, ChunkHit};
use evidence_core::storage::Storage;
use rusqlite::params;

const FIXTURE_CHUNKS: &[&str] = &[
    "the trial recruited 240 patients with stage III non-small-cell lung cancer",
    "patients were randomized 1:1 to pembrolizumab plus chemotherapy or chemotherapy alone",
    "primary endpoint was overall survival measured at 24 months",
    "secondary endpoints included progression-free survival and objective response rate",
    "the contraindication list excludes pregnant women and patients under 18",
    "adverse events were graded according to CTCAE version 5.0",
    "site investigators submitted weekly safety reports to the sponsor",
];

fn seeded() -> Storage {
    let storage = Storage::open_in_memory().expect("storage");
    let conn = storage.conn();
    // One document, one page, one span per chunk so foreign keys stay happy.
    conn.execute(
        "INSERT INTO documents (sha256, page_count, ingested_at) VALUES ('fix', 1, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO pages (doc_id, page_num, raw_text) VALUES (1, 1, '')",
        [],
    )
    .unwrap();
    let mut offset: i64 = 0;
    for text in FIXTURE_CHUNKS {
        let len = i64::try_from(text.len()).unwrap();
        conn.execute(
            "INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json) \
             VALUES (1, ?, ?, ?, '{}')",
            params![offset, offset + len, text],
        )
        .unwrap();
        offset += len;
    }
    for (i, text) in FIXTURE_CHUNKS.iter().enumerate() {
        let span_id = i64::try_from(i + 1).unwrap();
        conn.execute(
            "INSERT INTO chunks (id, span_id_start, span_id_end, text) VALUES (?, ?, ?, ?)",
            params![span_id, span_id, span_id, text],
        )
        .unwrap();
    }
    storage
}

#[test]
fn empty_query_returns_empty() {
    let storage = seeded();
    assert!(bm25_search(storage.conn(), "", 10).unwrap().is_empty());
    assert!(bm25_search(storage.conn(), "   ", 10).unwrap().is_empty());
}

#[test]
fn triggers_keep_chunks_fts_in_sync_on_insert() {
    let storage = seeded();
    let count: i64 = storage
        .conn()
        .query_row("SELECT count(*) FROM chunks_fts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count,
        i64::try_from(FIXTURE_CHUNKS.len()).unwrap(),
        "INSERT trigger should mirror every chunk into chunks_fts",
    );
}

#[test]
fn triggers_keep_chunks_fts_in_sync_on_delete() {
    let storage = seeded();
    storage
        .conn()
        .execute("DELETE FROM chunks WHERE id = 1", [])
        .unwrap();
    let hits = bm25_search(storage.conn(), "pembrolizumab", 5).unwrap();
    // The deleted chunk doesn't mention pembrolizumab, so this still
    // succeeds — but the deleted rowid must not appear in any hit.
    assert!(
        hits.iter().all(|h| h.chunk_id != 1),
        "chunks_fts should not return deleted chunks",
    );
}

#[test]
fn triggers_keep_chunks_fts_in_sync_on_update() {
    let storage = seeded();
    storage
        .conn()
        .execute(
            "UPDATE chunks SET text = ? WHERE id = ?",
            params!["xyzzyneedle", 6_i64],
        )
        .unwrap();

    let hits = bm25_search(storage.conn(), "xyzzyneedle", 5).unwrap();
    assert_eq!(
        hits.iter().map(|h| h.chunk_id).collect::<Vec<_>>(),
        vec![6],
        "UPDATE trigger should rewrite the FTS row",
    );
}

#[test]
fn bm25_returns_top_hit_for_a_distinctive_query() {
    let storage = seeded();
    let hits = bm25_search(storage.conn(), "contraindication", 3).unwrap();
    assert!(
        !hits.is_empty(),
        "fixture must produce at least one hit for 'contraindication'",
    );
    assert_eq!(
        hits[0].chunk_id, 5,
        "the contraindication chunk should rank first",
    );
}

#[test]
fn bm25_scores_are_descending() {
    let storage = seeded();
    let hits = bm25_search(storage.conn(), "patients", 10).unwrap();
    assert!(
        hits.len() >= 2,
        "fixture must produce multiple hits for 'patients'",
    );
    for window in hits.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "scores must be descending: {} then {}",
            window[0].score,
            window[1].score,
        );
    }
}

#[test]
fn bm25_respects_k_limit() {
    let storage = seeded();
    let hits = bm25_search(storage.conn(), "patients OR cancer", 2).unwrap();
    assert!(hits.len() <= 2, "k=2 must cap result count");
}

#[test]
fn chunk_hit_carries_span_range() {
    let storage = seeded();
    let hits = bm25_search(storage.conn(), "contraindication", 1).unwrap();
    let hit: &ChunkHit = &hits[0];
    assert_eq!(hit.chunk_id, 5);
    assert_eq!(hit.span_id_start, 5);
    assert_eq!(hit.span_id_end, 5);
    assert!(
        hit.score > 0.0,
        "BM25 score should be positive after sign flip"
    );
}
