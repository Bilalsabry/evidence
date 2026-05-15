//! Vector + hybrid retrieval tests. The embedder used here is a
//! deterministic mock that doesn't touch the network — these tests run
//! milliseconds. The real `BgeSmall` integration lives in
//! `tests/bge_small.rs` and runs end-to-end.

use evidence_core::retrieval::{
    hybrid_search, upsert_chunk_embedding, vector_search, EmbedError, Embedder,
};
use evidence_core::storage::Storage;
use rusqlite::params;

const DIM: usize = 384;

const CORPUS: &[(&str, [f32; 4])] = &[
    ("hello world", [1.0, 0.0, 0.0, 0.0]),
    ("evidence for pharma", [0.0, 1.0, 0.0, 0.0]),
    ("clinical trial protocol", [0.0, 0.0, 1.0, 0.0]),
    ("unrelated cat photos", [0.0, 0.0, 0.0, 1.0]),
];

fn pad_to_dim(prefix: [f32; 4]) -> Vec<f32> {
    let mut v = vec![0.0_f32; DIM];
    v[..4].copy_from_slice(&prefix);
    v
}

fn seed_corpus() -> Storage {
    let storage = Storage::open_in_memory().expect("storage");
    let conn = storage.conn();
    conn.execute(
        "INSERT INTO documents (sha256, page_count, ingested_at) VALUES ('vh', 1, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO pages (doc_id, page_num, raw_text) VALUES (1, 1, '')",
        [],
    )
    .unwrap();
    for (i, (text, _)) in CORPUS.iter().enumerate() {
        let id = i64::try_from(i + 1).unwrap();
        let len = i64::try_from(text.len()).unwrap();
        conn.execute(
            "INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json) \
             VALUES (1, ?, ?, ?, '{}')",
            params![id - 1, id - 1 + len, text],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chunks (id, span_id_start, span_id_end, text) VALUES (?, ?, ?, ?)",
            params![id, id, id, text],
        )
        .unwrap();
        upsert_chunk_embedding(conn, id, &pad_to_dim(CORPUS[i].1)).unwrap();
    }
    storage
}

#[test]
fn vector_search_ranks_exact_match_first() {
    let storage = seed_corpus();
    let probe = pad_to_dim(CORPUS[2].1); // "clinical trial protocol"
    let hits = vector_search(storage.conn(), &probe, 4).unwrap();
    assert_eq!(hits.len(), 4);
    assert_eq!(hits[0].chunk_id, 3, "exact-match vector ranks first");
}

#[test]
fn vector_search_respects_k() {
    let storage = seed_corpus();
    let probe = pad_to_dim([1.0, 0.0, 0.0, 0.0]);
    let hits = vector_search(storage.conn(), &probe, 2).unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn vector_search_empty_inputs_return_empty() {
    let storage = seed_corpus();
    assert!(vector_search(storage.conn(), &[], 5).unwrap().is_empty());
    let probe = pad_to_dim([1.0, 0.0, 0.0, 0.0]);
    assert!(vector_search(storage.conn(), &probe, 0).unwrap().is_empty());
}

#[test]
fn upsert_replaces_existing_embedding() {
    let storage = seed_corpus();
    // Overwrite chunk 1 to look like chunk 4.
    let chunk4_vec = pad_to_dim(CORPUS[3].1);
    upsert_chunk_embedding(storage.conn(), 1, &chunk4_vec).unwrap();
    let hits = vector_search(storage.conn(), &chunk4_vec, 2).unwrap();
    // Both chunk 1 (rewritten) and chunk 4 (original) should be in the top 2.
    let ids: Vec<i64> = hits.iter().map(|h| h.chunk_id).collect();
    assert!(ids.contains(&1));
    assert!(ids.contains(&4));
}

/// Mock embedder: maps every chunk's text back to the fixture vector it was
/// seeded with. Lets hybrid_search drive the same vector_search results
/// without touching a real model.
struct CorpusMockEmbedder;

impl Embedder for CorpusMockEmbedder {
    fn dim(&self) -> usize {
        DIM
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        texts
            .iter()
            .map(|t| {
                CORPUS
                    .iter()
                    .find(|(s, _)| *s == *t)
                    .map(|(_, prefix)| pad_to_dim(*prefix))
                    .ok_or_else(|| EmbedError::Run(format!("mock has no vector for: {t}")))
            })
            .collect()
    }
}

#[test]
fn hybrid_search_returns_results_from_both_retrievers() {
    let storage = seed_corpus();
    // FTS triggers populated chunks_fts on insert; vector index populated
    // via upsert_chunk_embedding. Both halves of the index are live.
    let embedder = CorpusMockEmbedder;
    let hits = hybrid_search(storage.conn(), &embedder, "clinical trial protocol", 4).unwrap();
    assert!(!hits.is_empty(), "hybrid must return at least one hit");
    assert_eq!(
        hits[0].chunk_id, 3,
        "the exact-match chunk should top the fused ranking",
    );
}

#[test]
fn hybrid_search_handles_empty_query() {
    let storage = seed_corpus();
    let embedder = CorpusMockEmbedder;
    assert!(hybrid_search(storage.conn(), &embedder, "   ", 4)
        .unwrap()
        .is_empty(),);
}
