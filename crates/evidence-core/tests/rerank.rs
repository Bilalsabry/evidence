//! Reranker pipeline tests. The mock reranker tests run in milliseconds;
//! the real-model integration test downloads `bge-reranker-base` on first
//! run (~280 MB, cached afterwards).

use std::sync::Mutex;

use evidence_core::retrieval::{
    hybrid_search_with_reranker, upsert_chunk_embedding, BgeReranker, EmbedError, Embedder,
    RerankError, RerankHit, Reranker,
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

fn seed() -> Storage {
    let storage = Storage::open_in_memory().expect("storage");
    let conn = storage.conn();
    conn.execute(
        "INSERT INTO documents (sha256, page_count, ingested_at) VALUES ('r', 1, 0)",
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

/// Embedder that maps known corpus strings back to their seeded vectors.
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
                    .map(|(_, p)| pad_to_dim(*p))
                    .ok_or_else(|| EmbedError::Run(format!("no fixture vector for: {t}")))
            })
            .collect()
    }
}

/// Reranker that prefers a specific chunk text by always scoring it highest,
/// regardless of the retriever's ranking. Lets us prove the reranker score
/// is what determines the final order.
struct PreferTextReranker {
    preferred: &'static str,
    calls: Mutex<usize>,
}
impl Reranker for PreferTextReranker {
    fn rerank(&self, _query: &str, documents: &[&str]) -> Result<Vec<RerankHit>, RerankError> {
        *self.calls.lock().unwrap() += 1;
        let mut scored: Vec<RerankHit> = documents
            .iter()
            .enumerate()
            .map(|(i, d)| RerankHit {
                index: i,
                score: if *d == self.preferred { 100.0 } else { 1.0 },
            })
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(scored)
    }
}

#[test]
fn reranker_reorders_the_fused_set() {
    let storage = seed();
    let embedder = CorpusMockEmbedder;
    let reranker = PreferTextReranker {
        preferred: "unrelated cat photos",
        calls: Mutex::new(0),
    };
    let hits = hybrid_search_with_reranker(
        storage.conn(),
        &embedder,
        &reranker,
        "clinical trial protocol",
        2,
    )
    .unwrap();
    assert_eq!(hits.len(), 2, "k=2 must cap the output");
    assert_eq!(
        hits[0].chunk_id, 4,
        "reranker preference must win over retriever ranking",
    );
    assert!(
        (hits[0].score - 100.0).abs() < 1e-6,
        "first-rank ChunkHit::score is the reranker score, got {}",
        hits[0].score,
    );
    assert_eq!(*reranker.calls.lock().unwrap(), 1);
}

#[test]
fn reranker_respects_k_truncation() {
    let storage = seed();
    let embedder = CorpusMockEmbedder;
    let reranker = PreferTextReranker {
        preferred: "evidence for pharma",
        calls: Mutex::new(0),
    };
    let hits = hybrid_search_with_reranker(
        storage.conn(),
        &embedder,
        &reranker,
        "evidence for pharma",
        1,
    )
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chunk_id, 2);
}

#[test]
fn reranker_handles_empty_query() {
    let storage = seed();
    let embedder = CorpusMockEmbedder;
    let reranker = PreferTextReranker {
        preferred: "anything",
        calls: Mutex::new(0),
    };
    let hits = hybrid_search_with_reranker(storage.conn(), &embedder, &reranker, "   ", 4).unwrap();
    assert!(hits.is_empty());
    assert_eq!(
        *reranker.calls.lock().unwrap(),
        0,
        "reranker must not be called when there are no candidates",
    );
}

#[test]
fn reranker_handles_k_zero() {
    let storage = seed();
    let embedder = CorpusMockEmbedder;
    let reranker = PreferTextReranker {
        preferred: "anything",
        calls: Mutex::new(0),
    };
    let hits =
        hybrid_search_with_reranker(storage.conn(), &embedder, &reranker, "anything", 0).unwrap();
    assert!(hits.is_empty());
    assert_eq!(*reranker.calls.lock().unwrap(), 0);
}

/// Real-model integration. Marked separately so the CI cache key
/// (`fastembed-bge-small-en-v1.5-v1` in `.github/workflows/ci.yml`) can be
/// extended to also cache `bge-reranker-base` once we touch that file.
#[test]
fn bge_reranker_orders_obvious_relevance() {
    let reranker = BgeReranker::new().expect("bge-reranker-base init");
    let docs = [
        "Patients on pembrolizumab showed improved overall survival.",
        "The catalog of unrelated trivia includes the colors of fishbowls.",
        "Pembrolizumab is approved for non-small-cell lung cancer.",
    ];
    let hits = reranker
        .rerank("pembrolizumab in lung cancer", &docs)
        .unwrap();
    assert_eq!(hits.len(), 3);
    // Indices 0 and 2 are clearly more relevant than 1.
    assert!(
        hits[0].index != 1 && hits[1].index != 1,
        "trivia document must not rank top-2; got order: {hits:?}",
    );
    assert_eq!(hits[2].index, 1, "trivia document must rank last");
    for w in hits.windows(2) {
        assert!(w[0].score >= w[1].score, "scores must be descending");
    }
}
