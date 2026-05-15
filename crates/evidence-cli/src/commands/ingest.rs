//! `evidence ingest <path>`: thin wrapper around
//! [`evidence_core::ingest::ingest_pdf`].

use std::path::Path;

use evidence_core::ingest::{ingest_pdf, IngestError, IngestSummary};
use evidence_core::retrieval::Embedder;
use evidence_core::storage::Storage;

/// Run the ingest command. Returns the [`IngestSummary`] for the caller to
/// render however it wants.
///
/// # Errors
///
/// Propagates any [`IngestError`] from the core pipeline.
pub fn run(
    storage: &mut Storage,
    embedder: &dyn Embedder,
    path: &Path,
    title: Option<&str>,
) -> Result<IngestSummary, IngestError> {
    ingest_pdf(storage, embedder, path, title)
}
