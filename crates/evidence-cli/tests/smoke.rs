//! End-to-end smoke test for the CLI pipeline. Runs entirely in-process
//! with a generated fixture PDF, an in-memory SQLite store, a deterministic
//! hash embedder, and the `MockBackend` from `evidence_core::query::testing`.
//! No network. No model downloads. No real LLM.

use std::io::Write;

use evidence_cli::commands::{ingest, query};
use evidence_core::query::testing::{
    ApprovingSupport, MockBackend, OutOfContextBackend, RefusingBackend, RejectingSupport,
    UncitedBackend,
};
use evidence_core::query::{LlmBackend, QueryError, SupportChecker};
use evidence_core::retrieval::{EmbedError, Embedder};
use evidence_core::storage::Storage;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use tempfile::NamedTempFile;

const SMOKE_PAGE_TEXTS: [&str; 3] = [
    "Evidence is an auditable research assistant.",
    "Every sentence is hyperlinked to a span.",
    "If the model cannot cite, it refuses to answer.",
];

const DIM: usize = 384;

/// Deterministic embedder used so tests don't need to download a model.
/// One dimension per byte (mod DIM) — sufficient for vector_search to
/// produce stable rankings against itself.
struct HashEmbedder;

impl Embedder for HashEmbedder {
    fn dim(&self) -> usize {
        DIM
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| {
                let mut v = vec![0.0_f32; DIM];
                for (i, b) in t.bytes().enumerate() {
                    v[i % DIM] += (f32::from(b) - 128.0) / 128.0;
                }
                v
            })
            .collect())
    }
}

fn fixture_pdf_bytes() -> Vec<u8> {
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let pages = Ref::new(2);
    let font = Ref::new(3);
    let page_ids = [Ref::new(10), Ref::new(20), Ref::new(30)];
    let content_ids = [Ref::new(11), Ref::new(21), Ref::new(31)];

    pdf.catalog(catalog).pages(pages);
    pdf.pages(pages).count(3).kids(page_ids.iter().copied());
    pdf.type1_font(font).base_font(Name(b"Helvetica"));

    for ((page_id, content_id), text) in page_ids
        .iter()
        .copied()
        .zip(content_ids.iter().copied())
        .zip(SMOKE_PAGE_TEXTS.iter())
    {
        let mut page = pdf.page(page_id);
        page.parent(pages)
            .media_box(Rect::new(0.0, 0.0, 612.0, 792.0))
            .contents(content_id)
            .resources()
            .fonts()
            .pair(Name(b"F1"), font);
        page.finish();

        let mut content = Content::new();
        content
            .begin_text()
            .set_font(Name(b"F1"), 24.0)
            .next_line(72.0, 720.0)
            .show(Str(text.as_bytes()))
            .end_text();
        pdf.stream(content_id, &content.finish());
    }
    pdf.finish()
}

fn write_pdf() -> NamedTempFile {
    let mut f = NamedTempFile::new().expect("tempfile");
    f.write_all(&fixture_pdf_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn fresh_storage() -> Storage {
    Storage::open_in_memory().expect("storage")
}

fn ingested() -> (Storage, HashEmbedder) {
    let mut storage = fresh_storage();
    let embedder = HashEmbedder;
    let pdf = write_pdf();
    let summary = ingest::run(&mut storage, &embedder, pdf.path(), Some("Smoke fixture"))
        .expect("ingest should succeed");
    assert_eq!(summary.page_count, 3);
    assert!(summary.chunk_count >= 1, "chunker must emit ≥1 chunk");
    (storage, embedder)
}

#[test]
fn ingest_then_query_with_mock_backend_returns_an_answer() {
    let (storage, embedder) = ingested();
    let llm: Box<dyn LlmBackend> = Box::new(MockBackend::new());
    let answer = query::run(
        &storage,
        &embedder,
        llm.as_ref(),
        None,
        "what is evidence?",
        4,
    )
    .expect("query should succeed under the mock backend");

    assert!(!answer.sentences.is_empty());
    let s = &answer.sentences[0];
    assert!(
        !s.citations.is_empty(),
        "every validated sentence has at least one citation",
    );

    let formatted = query::format_answer(&answer);
    assert!(
        formatted.lines().any(|line| line.contains("[p")),
        "rendered output should carry [pN:spanM] citation markers, got: {formatted}",
    );
}

#[test]
fn refusing_backend_produces_refusal_message() {
    let (storage, embedder) = ingested();
    let llm: Box<dyn LlmBackend> = Box::new(RefusingBackend);
    let err = query::run(&storage, &embedder, llm.as_ref(), None, "anything", 4)
        .expect_err("RefusingBackend must surface as QueryError");
    let refusal = query::format_refusal(&err);
    assert!(
        refusal.contains("I can't answer"),
        "refusal text must be the standard refusal: {refusal}",
    );
}

#[test]
fn uncited_sentence_is_rejected_by_validator() {
    let (storage, embedder) = ingested();
    let llm: Box<dyn LlmBackend> = Box::new(UncitedBackend);
    let err = query::run(&storage, &embedder, llm.as_ref(), None, "anything", 4)
        .expect_err("uncited sentence must be rejected");
    assert!(matches!(err, QueryError::Uncited { .. }));
}

#[test]
fn out_of_context_citation_is_rejected_by_validator() {
    let (storage, embedder) = ingested();
    // 999_999 won't appear in any chunk's span range.
    let llm: Box<dyn LlmBackend> = Box::new(OutOfContextBackend {
        bogus_span_id: 999_999,
    });
    let err = query::run(&storage, &embedder, llm.as_ref(), None, "anything", 4)
        .expect_err("bogus span_id must be rejected");
    assert!(
        matches!(err, QueryError::OutOfContext { .. }),
        "expected OutOfContext, got {err:?}",
    );
}

#[test]
fn approving_support_check_passes_through() {
    let (storage, embedder) = ingested();
    let llm: Box<dyn LlmBackend> = Box::new(MockBackend::new());
    let support: Box<dyn SupportChecker> = Box::new(ApprovingSupport);
    let answer = query::run(
        &storage,
        &embedder,
        llm.as_ref(),
        Some(support.as_ref()),
        "what is evidence?",
        4,
    )
    .expect("ApprovingSupport must not block the validated answer");
    assert!(!answer.sentences.is_empty());
}

#[test]
fn rejecting_support_check_blocks_otherwise_valid_answer() {
    let (storage, embedder) = ingested();
    let llm: Box<dyn LlmBackend> = Box::new(MockBackend::new());
    let support: Box<dyn SupportChecker> = Box::new(RejectingSupport);
    let err = query::run(
        &storage,
        &embedder,
        llm.as_ref(),
        Some(support.as_ref()),
        "what is evidence?",
        4,
    )
    .expect_err("RejectingSupport must turn a clean answer into a refusal");
    assert!(
        matches!(err, QueryError::Unsupported { .. }),
        "expected Unsupported, got {err:?}",
    );
}
