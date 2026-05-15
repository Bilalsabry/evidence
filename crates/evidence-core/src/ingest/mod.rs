//! Source-document ingestion.
//!
//! Each ingest backend turns a file on disk into a sequence of [`pdf::Page`]
//! records made up of byte-addressed [`pdf::Span`] records. Storage and
//! retrieval consume those records; the ingest layer is otherwise stateless.

pub mod pdf;
pub mod pipeline;

pub use pipeline::{
    ingest_pdf, ingest_pdf_with_progress, IngestError, IngestProgress, IngestStage, IngestSummary,
    NoIngestProgress, MAX_CHUNK_BYTES,
};
