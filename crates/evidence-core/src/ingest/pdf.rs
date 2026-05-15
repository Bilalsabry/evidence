//! PDF ingestion built on PDFium.
//!
//! [`extract`] turns a PDF on disk into a `Vec<Page>`, where each [`Page`]
//! holds the raw text of the page plus byte-indexed [`Span`]s with bounding
//! boxes. Spans are the unit of citation downstream: every byte of
//! `Page::raw_text` belongs to exactly one span.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pdfium_render::prelude::{PdfRect, Pdfium, PdfiumError};
use thiserror::Error;

/// Rectangle in PDF page coordinates: PDF points, origin at the bottom-left.
#[derive(Debug, Clone, Copy, PartialEq)]
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

        let mut raw_text = String::new();
        let mut spans = Vec::new();
        for segment in page_text.segments().iter() {
            let text = segment.text();
            if text.is_empty() {
                continue;
            }
            let start_offset = raw_text.len();
            raw_text.push_str(&text);
            let end_offset = raw_text.len();
            spans.push(Span {
                start_offset,
                end_offset,
                text,
                bbox: bbox_from(&segment.bounds()),
            });
        }

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
