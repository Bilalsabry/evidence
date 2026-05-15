//! End-to-end storage tests: schema apply, idempotent migrations, round-trip
//! reads/writes for every table, and a single-connection check that FTS5
//! and `vec0` are both live.

use evidence_core::storage::{migrations, Storage};
use rusqlite::params;

fn fresh() -> Storage {
    Storage::open_in_memory().expect("fresh storage")
}

fn schema_version(storage: &Storage) -> i32 {
    storage
        .conn()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("user_version")
}

#[test]
fn fresh_db_has_latest_schema_version() {
    let storage = fresh();
    assert_eq!(schema_version(&storage), migrations::latest_version());
}

#[test]
fn migrate_is_idempotent() {
    let storage = fresh();
    let before = schema_version(&storage);
    migrations::migrate(storage.conn()).expect("re-migrate");
    migrations::migrate(storage.conn()).expect("re-migrate twice");
    let after = schema_version(&storage);
    assert_eq!(before, after);
    assert_eq!(after, migrations::latest_version());
}

#[test]
fn documents_roundtrip() {
    let storage = fresh();
    storage
        .conn()
        .execute(
            "INSERT INTO documents (sha256, title, page_count, ingested_at) \
             VALUES (?, ?, ?, ?)",
            params!["abc123", "Trial Protocol v1", 12, 1_700_000_000_000_i64],
        )
        .expect("insert document");

    let (sha, title, pages, ts): (String, Option<String>, i64, i64) = storage
        .conn()
        .query_row(
            "SELECT sha256, title, page_count, ingested_at FROM documents",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read document");

    assert_eq!(sha, "abc123");
    assert_eq!(title.as_deref(), Some("Trial Protocol v1"));
    assert_eq!(pages, 12);
    assert_eq!(ts, 1_700_000_000_000);
}

#[test]
fn pages_roundtrip_under_foreign_key() {
    let storage = fresh();
    storage
        .conn()
        .execute(
            "INSERT INTO documents (sha256, page_count, ingested_at) VALUES (?, ?, ?)",
            params!["doc-fk", 3, 0_i64],
        )
        .unwrap();
    let doc_id: i64 = storage
        .conn()
        .query_row(
            "SELECT id FROM documents WHERE sha256 = ?",
            ["doc-fk"],
            |r| r.get(0),
        )
        .unwrap();

    storage
        .conn()
        .execute(
            "INSERT INTO pages (doc_id, page_num, raw_text) VALUES (?, ?, ?)",
            params![doc_id, 1_i64, "page one"],
        )
        .unwrap();

    let count: i64 = storage
        .conn()
        .query_row(
            "SELECT count(*) FROM pages WHERE doc_id = ?",
            [doc_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // FK cascade: deleting the document should remove the page.
    storage
        .conn()
        .execute("DELETE FROM documents WHERE id = ?", [doc_id])
        .unwrap();
    let after: i64 = storage
        .conn()
        .query_row(
            "SELECT count(*) FROM pages WHERE doc_id = ?",
            [doc_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after, 0, "FK cascade should delete the page");
}

#[test]
fn spans_roundtrip_with_bbox_json() {
    let storage = fresh();
    storage
        .conn()
        .execute_batch(
            "INSERT INTO documents (sha256, page_count, ingested_at) VALUES ('s', 1, 0);
             INSERT INTO pages (doc_id, page_num, raw_text) VALUES (1, 1, 'hello');",
        )
        .unwrap();

    let bbox = r#"{"x0":10.0,"y0":20.0,"x1":110.0,"y1":40.0}"#;
    storage
        .conn()
        .execute(
            "INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json) \
             VALUES (1, 0, 5, 'hello', ?)",
            [bbox],
        )
        .unwrap();

    let (text, bbox_out): (String, String) = storage
        .conn()
        .query_row("SELECT text, bbox_json FROM spans WHERE id = 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(text, "hello");
    assert_eq!(bbox_out, bbox);
}

#[test]
fn chunks_roundtrip() {
    let storage = fresh();
    storage
        .conn()
        .execute_batch(
            "INSERT INTO documents (sha256, page_count, ingested_at) VALUES ('c', 1, 0);
             INSERT INTO pages (doc_id, page_num, raw_text) VALUES (1, 1, 'x');
             INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json)
               VALUES (1, 0, 1, 'x', '{}');
             INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json)
               VALUES (1, 1, 2, 'y', '{}');",
        )
        .unwrap();

    storage
        .conn()
        .execute(
            "INSERT INTO chunks (span_id_start, span_id_end, text) VALUES (?, ?, ?)",
            params![1_i64, 2_i64, "xy"],
        )
        .unwrap();

    let (start, end, text): (i64, i64, String) = storage
        .conn()
        .query_row(
            "SELECT span_id_start, span_id_end, text FROM chunks",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!((start, end), (1, 2));
    assert_eq!(text, "xy");
}

#[test]
fn fts5_and_vec0_live_on_the_same_connection() {
    let storage = fresh();
    let conn = storage.conn();

    // Seed enough rows for a meaningful match in both indexes.
    conn.execute_batch(
        "INSERT INTO documents (sha256, page_count, ingested_at) VALUES ('d', 1, 0);
         INSERT INTO pages (doc_id, page_num, raw_text) VALUES (1, 1, 'hello world');
         INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json)
           VALUES (1, 0, 5,  'hello', '{}'),
                  (1, 6, 11, 'world', '{}');
         INSERT INTO chunks (id, span_id_start, span_id_end, text)
           VALUES (1, 1, 2, 'auditable evidence for pharma'),
                  (2, 2, 2, 'completely unrelated cat photos');",
    )
    .unwrap();

    // FTS5: content-linked table, so mirror inserts manually for now.
    conn.execute(
        "INSERT INTO chunks_fts (rowid, text) VALUES (1, 'auditable evidence for pharma'), \
         (2, 'completely unrelated cat photos')",
        [],
    )
    .unwrap();

    let fts_hits: Vec<i64> = conn
        .prepare("SELECT rowid FROM chunks_fts WHERE chunks_fts MATCH 'auditable' ORDER BY rowid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(fts_hits, vec![1], "FTS5 must isolate the auditable chunk");

    // vec0: insert two 384-dim vectors. Use a JSON-array literal — vec0
    // accepts that as input.
    let near: Vec<f32> = (0..384).map(|i| (i as f32) * 0.001).collect();
    let far: Vec<f32> = (0..384).map(|i| ((i as f32) * 0.001) + 5.0).collect();
    let near_json = vec_to_json(&near);
    let far_json = vec_to_json(&far);

    conn.execute(
        "INSERT INTO chunks_vec (rowid, embedding) VALUES (1, ?), (2, ?)",
        params![near_json, far_json],
    )
    .unwrap();

    // KNN query: the `near` vector should rank first against itself.
    let probe = vec_to_json(&near);
    let knn: Vec<(i64, f64)> = conn
        .prepare(
            "SELECT rowid, distance FROM chunks_vec WHERE embedding MATCH ? ORDER BY distance LIMIT 2",
        )
        .unwrap()
        .query_map([probe], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(knn.len(), 2);
    assert_eq!(knn[0].0, 1, "exact-match vector should be top-ranked");
    assert!(
        knn[0].1 < knn[1].1,
        "distances must be ordered: {} < {}",
        knn[0].1,
        knn[1].1
    );
}

fn vec_to_json(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 8);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // Use a fixed format so the JSON parser inside vec0 is happy with
        // every value, including negatives and zero.
        s.push_str(&format!("{x}"));
    }
    s.push(']');
    s
}
