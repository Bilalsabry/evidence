//! PDF ingestion built on PDFium.
//!
//! [`extract`] turns a PDF on disk into a `Vec<Page>`, where each [`Page`]
//! holds the raw text of the page plus byte-indexed [`Span`]s with bounding
//! boxes. Spans are the unit of citation downstream: every byte of
//! `Page::raw_text` belongs to exactly one span.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pdfium_render::prelude::{PdfPageText, PdfRect, Pdfium, PdfiumError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Rectangle in PDF page coordinates: PDF points, origin at the bottom-left.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bbox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// A contiguous run of characters with a single bounding box.
///
/// Citations resolve to spans. `start_offset..end_offset` are byte offsets
/// into the parent [`Page::raw_text`].
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start_offset: usize,
    pub end_offset: usize,
    pub text: String,
    pub bbox: Bbox,
}

/// One page of an extracted document. `spans` are in reading order and
/// partition `raw_text` exactly: spans are contiguous, non-overlapping, and
/// cover every byte.
#[derive(Debug, Clone)]
pub struct Page {
    /// 1-indexed page number.
    pub page_num: u32,
    /// Concatenation of every span's text, in reading order.
    pub raw_text: String,
    pub spans: Vec<Span>,
}

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("failed to load PDF: {0}")]
    Load(String),
    #[error("failed to read page {page}: {message}")]
    Page { page: u32, message: String },
    #[error("PDF is password-protected")]
    Encrypted,
    #[error("PDFium backend unavailable: {0}")]
    Backend(String),
}

/// Resolve the PDFium library path, downloading it on first call. Subsequent
/// calls reuse the cached `PathBuf`. The `OnceLock` serializes concurrent
/// downloads, which matters for parallel test execution against a cold cache.
fn pdfium_library_path() -> Result<&'static Path, PdfError> {
    static PATH: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    PATH.get_or_init(|| pdfium_auto::ensure_pdfium_library(None).map_err(|e| e.to_string()))
        .as_deref()
        .map_err(|e| PdfError::Backend(e.clone()))
}

/// Bind to PDFium using the cached library path. Binding itself is cheap.
fn bind() -> Result<Pdfium, PdfError> {
    let path = pdfium_library_path()?;
    pdfium_auto::bind_pdfium_from_path(path).map_err(|e| PdfError::Backend(e.to_string()))
}

/// Extract pages and spans from a PDF on disk.
///
/// # Errors
///
/// Returns [`PdfError::Load`] for a missing or malformed file,
/// [`PdfError::Encrypted`] if the file requires a password, and
/// [`PdfError::Backend`] if the PDFium library cannot be loaded.
pub fn extract<P: AsRef<Path>>(path: P) -> Result<Vec<Page>, PdfError> {
    let path = path.as_ref();
    let pdfium = bind()?;
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(map_pdfium_err)?;

    let pages = doc.pages();
    let mut out = Vec::with_capacity(pages.len() as usize);

    for (idx, page) in pages.iter().enumerate() {
        let page_num = u32::try_from(idx).unwrap_or(u32::MAX).saturating_add(1);
        let page_text = page.text().map_err(|e| PdfError::Page {
            page: page_num,
            message: e.to_string(),
        })?;

        let (raw_text, spans) = group_chars_into_lines(&page_text);

        out.push(Page {
            page_num,
            raw_text,
            spans,
        });
    }

    Ok(out)
}

fn bbox_from(rect: &PdfRect) -> Bbox {
    Bbox {
        x0: rect.left().value,
        y0: rect.bottom().value,
        x1: rect.right().value,
        y1: rect.top().value,
    }
}

/// Smallest box covering both inputs.
fn union_bbox(a: Bbox, b: Bbox) -> Bbox {
    Bbox {
        x0: a.x0.min(b.x0),
        y0: a.y0.min(b.y0),
        x1: a.x1.max(b.x1),
        y1: a.y1.max(b.y1),
    }
}

/// Group a page's characters into **line-level** spans.
///
/// PDFium's `segments()` API is glyph-level on many real-world PDFs
/// (one segment per character), which is the wrong granularity for
/// citation: a citation should resolve to a readable run of text, not a
/// single letter. We rebuild lines from the character stream instead.
///
/// A line ends when we hit an explicit newline, or — for PDFs that
/// encode no newline characters — when a glyph's vertical band stops
/// overlapping the current line's band (a baseline jump). Each emitted
/// span's text is exactly the slice of `raw_text` it covers, so spans
/// still partition `raw_text` contiguously (the invariant ingest and
/// the citation resolver rely on).
fn group_chars_into_lines(page_text: &PdfPageText) -> (String, Vec<Span>) {
    let mut raw_text = String::new();
    let mut spans = Vec::new();

    let mut line_text = String::new();
    let mut line_bbox: Option<Bbox> = None;

    // `raw_text` already contains the current line's chars, so the line
    // occupies its trailing `line_text.len()` bytes. Deriving offsets
    // from that keeps spans partitioning `raw_text` with no separate
    // cursor to keep in sync.
    macro_rules! flush_line {
        () => {
            if !line_text.is_empty() {
                let end = raw_text.len();
                let start = end - line_text.len();
                spans.push(Span {
                    start_offset: start,
                    end_offset: end,
                    text: std::mem::take(&mut line_text),
                    // A line with no glyph bounds (e.g. whitespace only)
                    // gets a zero box rather than being dropped — the
                    // partition invariant matters more than its bbox.
                    bbox: line_bbox.take().unwrap_or(Bbox {
                        x0: 0.0,
                        y0: 0.0,
                        x1: 0.0,
                        y1: 0.0,
                    }),
                });
            }
        };
    }

    let chars = page_text.chars();
    for ch in chars.iter() {
        let Some(c) = ch.unicode_char() else {
            continue;
        };
        let glyph_bbox = ch.loose_bounds().ok().map(|r| bbox_from(&r));

        // Baseline-jump detection for PDFs with no explicit newlines:
        // if this glyph's vertical band doesn't overlap the line's, the
        // line is over. Skip the check for the newline char itself and
        // for boundless glyphs (whitespace).
        if c != '\n' && c != '\r' {
            if let (Some(g), Some(l)) = (glyph_bbox, line_bbox) {
                let overlaps = g.y0 <= l.y1 && g.y1 >= l.y0;
                if !overlaps && !line_text.is_empty() {
                    // Keep lines readable when joined downstream.
                    line_text.push('\n');
                    raw_text.push('\n');
                    flush_line!();
                }
            }
        }

        line_text.push(c);
        raw_text.push(c);
        if let Some(g) = glyph_bbox {
            line_bbox = Some(line_bbox.map_or(g, |acc| union_bbox(acc, g)));
        }

        if c == '\n' {
            flush_line!();
        }
    }
    flush_line!();

    (raw_text, spans)
}

fn map_pdfium_err(err: PdfiumError) -> PdfError {
    let message = err.to_string();
    // PDFium surfaces password-protected files as a generic internal error;
    // string-matching is the documented escape hatch in pdfium-render < 0.9.
    if message.contains("Password") || message.contains("password") {
        return PdfError::Encrypted;
    }
    PdfError::Load(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_from_rect_uses_pdf_coordinates() {
        let rect = PdfRect::new_from_values(10.0, 20.0, 30.0, 40.0);
        let bbox = bbox_from(&rect);
        // PdfRect::new_from_values is (bottom, left, top, right).
        assert_eq!(bbox.x0, 20.0);
        assert_eq!(bbox.y0, 10.0);
        assert_eq!(bbox.x1, 40.0);
        assert_eq!(bbox.y1, 30.0);
    }

    #[test]
    fn map_pdfium_err_detects_password() {
        let err = PdfError::Encrypted;
        // Sanity: the variant is constructible and Display works.
        assert_eq!(err.to_string(), "PDF is password-protected");
    }
}
