//! Real `BgeSmall` integration test. First run downloads the ONNX model
//! (~130 MB) from the Hugging Face hub; subsequent runs hit the cache.
//!
//! Asserts:
//! - The embedder reports the documented dimension.
//! - Single-text and batch-text outputs are consistent.
//! - The output for a fixture string is normalized (unit-ish length) and
//!   non-degenerate.
//!
//! We intentionally do *not* snapshot-test raw values: model files and ORT
//! kernels can change between releases. Asserting structural invariants
//! catches regressions without locking us to a specific build of bge-small.

use evidence_core::retrieval::{BgeSmall, Embedder, BGE_SMALL_DIM};

#[test]
fn bge_small_embeds_a_fixture_string() {
    let model = BgeSmall::new().expect("bge-small init (downloads model on first run)");
    assert_eq!(model.dim(), BGE_SMALL_DIM);

    let v = model.embed("evidence for pharma").expect("embed");
    assert_eq!(v.len(), BGE_SMALL_DIM);

    // bge-small is L2-normalized: ||v|| should be close to 1.
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (0.95..=1.05).contains(&norm),
        "expected unit-ish vector, got norm = {norm}",
    );

    // Output must not be all-zero (would mean the model failed to load).
    assert!(v.iter().any(|x| x.abs() > 1e-6));
}

#[test]
fn bge_small_batch_matches_singles() {
    let model = BgeSmall::new().expect("bge-small init");

    let single_a = model.embed("auditable").unwrap();
    let single_b = model.embed("evidence").unwrap();
    let batch = model.embed_batch(&["auditable", "evidence"]).unwrap();

    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0].len(), BGE_SMALL_DIM);
    assert_eq!(batch[1].len(), BGE_SMALL_DIM);

    for (a, b) in single_a.iter().zip(batch[0].iter()) {
        assert!(
            (a - b).abs() < 1e-4,
            "single vs batch[0] divergence: {a} vs {b}"
        );
    }
    for (a, b) in single_b.iter().zip(batch[1].iter()) {
        assert!(
            (a - b).abs() < 1e-4,
            "single vs batch[1] divergence: {a} vs {b}"
        );
    }
}
