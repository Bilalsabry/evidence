//! End-to-end ingest tests against a deterministic, generated PDF.
//!
//! The fixture is a single-column 3-page PDF written via `pdf-writer`. Each
//! page contains one short line of text. Tests assert page count, span
//! coverage of `raw_text`, and typed errors for malformed input.

use std::io::Write;

use evidence_core::ingest::pdf::{self, PdfError};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use tempfile::NamedTempFile;

const PAGE_TEXTS: [&str; 3] = [
    "Evidence page one heading.",
    "Second page contains text.",
    "Page three is the final page.",
];

/// Build a deterministic 3-page PDF in memory.
fn build_sample_pdf() -> Vec<u8> {
    let mut pdf = Pdf::new();

    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let font_id = Ref::new(3);
    let page_ids = [Ref::new(10), Ref::new(20), Ref::new(30)];
    let content_ids = [Ref::new(11), Ref::new(21), Ref::new(31)];

    pdf.catalog(catalog_id).pages(page_tree_id);

    pdf.pages(page_tree_id)
        .count(3)
        .kids(page_ids.iter().copied());

    pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

    for ((page_id, content_id), text) in page_ids
        .iter()
        .copied()
        .zip(content_ids.iter().copied())
        .zip(PAGE_TEXTS.iter())
    {
        let mut page = pdf.page(page_id);
        page.parent(page_tree_id)
            .media_box(Rect::new(0.0, 0.0, 612.0, 792.0))
            .contents(content_id)
            .resources()
            .fonts()
            .pair(Name(b"F1"), font_id);
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

fn write_to_tempfile(bytes: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("tempfile");
    file.write_all(bytes).expect("write");
    file.flush().expect("flush");
    file
}

#[test]
fn extracts_three_pages_with_text() {
    let file = write_to_tempfile(&build_sample_pdf());
    let pages = pdf::extract(file.path()).expect("extract should succeed");

    assert_eq!(pages.len(), 3, "page count");
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.page_num, u32::try_from(i).unwrap() + 1);
        assert!(
            !page.raw_text.is_empty(),
            "page {} has empty text",
            page.page_num
        );
        assert!(
            !page.spans.is_empty(),
            "page {} has zero spans",
            page.page_num
        );
    }
}

#[test]
fn spans_partition_raw_text() {
    let file = write_to_tempfile(&build_sample_pdf());
    let pages = pdf::extract(file.path()).expect("extract should succeed");

    for page in &pages {
        // First span starts at byte 0.
        assert_eq!(page.spans.first().unwrap().start_offset, 0);
        // Last span ends at raw_text length.
        assert_eq!(page.spans.last().unwrap().end_offset, page.raw_text.len());
        // Spans are contiguous and non-overlapping.
        for window in page.spans.windows(2) {
            assert_eq!(window[0].end_offset, window[1].start_offset);
        }
        // Every span's recorded text equals the slice of raw_text it covers.
        for span in &page.spans {
            assert_eq!(
                span.text,
                &page.raw_text[span.start_offset..span.end_offset]
            );
        }
    }
}

#[test]
fn malformed_input_returns_typed_error() {
    let file = write_to_tempfile(b"this is not a PDF, it's just bytes\n");
    let err = pdf::extract(file.path()).expect_err("malformed input should error");
    assert!(
        matches!(err, PdfError::Load(_)),
        "expected PdfError::Load, got {err:?}"
    );
}

#[test]
fn missing_file_returns_typed_error() {
    let mut path = std::env::temp_dir();
    path.push("evidence-no-such-file-xyz.pdf");
    let err = pdf::extract(&path).expect_err("missing file should error");
    assert!(matches!(err, PdfError::Load(_)));
}
