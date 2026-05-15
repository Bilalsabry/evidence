//! End-to-end ingest pipeline: take a PDF on disk, write its pages, spans,
//! chunks, and chunk embeddings to [`Storage`].
//!
//! The chunker is intentionally simple for v0.1: contiguous spans on the
//! same page are concatenated until they cross [`MAX_CHUNK_BYTES`], then a
//! new chunk starts. This is enough granularity for BM25 + vector to
//! produce useful hits without dragging in a tokenization library.

use std::path::Path;
use std::time::SystemTime;

use rusqlite::params;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ingest::pdf::{self, Page, PdfError, Span};
use crate::retrieval::{upsert_chunk_embedding, EmbedError, Embedder};
use crate::storage::{Storage, StorageError};

/// Soft byte cap per chunk. Picked to fit comfortably inside bge-small's
/// 512-token context after tokenization (most English text averages ~4 chars
/// per token).
pub const MAX_CHUNK_BYTES: usize = 1024;

/// Summary of an ingest run, returned to the caller for logging or display.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestSummary {
    pub document_id: i64,
    pub sha256: String,
    pub page_count: usize,
    pub span_count: usize,
    pub chunk_count: usize,
}

/// Errors surfaced by [`ingest_pdf`].
#[derive(Debug, Error)]
pub enum IngestError {
    #[error(transparent)]
    Pdf(#[from] PdfError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Embed(#[from] EmbedError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("document with sha256 {sha} already ingested")]
    Duplicate { sha: String },
}

impl From<StorageError> for IngestError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Sqlite(e) => IngestError::Sqlite(e),
            other => IngestError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::other(other.to_string()),
            ))),
        }
    }
}

/// Ingest a PDF end-to-end. Idempotent against re-ingestion: a document
/// already present (matched by SHA-256) errors with [`IngestError::Duplicate`]
/// instead of silently rewriting.
///
/// # Errors
///
/// Propagates [`PdfError`] from extraction, [`StorageError`] from the schema
/// write path, and [`EmbedError`] from the embedder.
pub fn ingest_pdf<P: AsRef<Path>>(
    storage: &mut Storage,
    embedder: &dyn Embedder,
    path: P,
    title: Option<&str>,
) -> Result<IngestSummary, IngestError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let sha = sha256_hex(&bytes);

    // Refuse to ingest the same document twice — keeps the SHA-unique index
    // honest and gives the caller a clear signal.
    if let Some(existing) = lookup_document_by_sha(storage, &sha)? {
        return Err(IngestError::Duplicate { sha: existing });
    }

    let pages = pdf::extract(path)?;
    let now_ms = i64::try_from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
    )
    .unwrap_or(i64::MAX);

    let tx = storage.conn_mut().transaction()?;
    let doc_id = {
        tx.execute(
            "INSERT INTO documents (sha256, title, page_count, ingested_at) \
             VALUES (?, ?, ?, ?)",
            params![&sha, title, pages.len() as i64, now_ms],
        )?;
        tx.last_insert_rowid()
    };

    let (span_count, chunks) = write_pages_spans_chunks(&tx, doc_id, &pages)?;

    // Embed all chunks in one batch. fastembed parallelizes internally; one
    // batch call is much faster than one-per-chunk.
    let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
    let embeddings = if texts.is_empty() {
        Vec::new()
    } else {
        embedder.embed_batch(&texts)?
    };

    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        upsert_chunk_embedding(&tx, chunk.id, embedding).map_err(|e| match e {
            crate::retrieval::RetrievalError::Sqlite(e) => IngestError::Sqlite(e),
        })?;
    }

    tx.commit()?;

    Ok(IngestSummary {
        document_id: doc_id,
        sha256: sha,
        page_count: pages.len(),
        span_count,
        chunk_count: chunks.len(),
    })
}

fn lookup_document_by_sha(storage: &Storage, sha: &str) -> Result<Option<String>, IngestError> {
    match storage.conn().query_row::<String, _, _>(
        "SELECT sha256 FROM documents WHERE sha256 = ?",
        [sha],
        |r| r.get(0),
    ) {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(other) => Err(IngestError::Sqlite(other)),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

struct InsertedChunk {
    id: i64,
    text: String,
}

fn write_pages_spans_chunks(
    tx: &rusqlite::Transaction<'_>,
    doc_id: i64,
    pages: &[Page],
) -> Result<(usize, Vec<InsertedChunk>), IngestError> {
    let mut total_spans = 0usize;
    let mut chunks: Vec<InsertedChunk> = Vec::new();

    for page in pages {
        tx.execute(
            "INSERT INTO pages (doc_id, page_num, raw_text) VALUES (?, ?, ?)",
            params![doc_id, page.page_num as i64, &page.raw_text],
        )?;
        let page_id = tx.last_insert_rowid();

        let mut span_ids: Vec<i64> = Vec::with_capacity(page.spans.len());
        for span in &page.spans {
            let bbox_json = serde_json::to_string(&span.bbox).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO spans (page_id, start_offset, end_offset, text, bbox_json) \
                 VALUES (?, ?, ?, ?, ?)",
                params![
                    page_id,
                    span.start_offset as i64,
                    span.end_offset as i64,
                    &span.text,
                    bbox_json,
                ],
            )?;
            span_ids.push(tx.last_insert_rowid());
        }
        total_spans += page.spans.len();

        // Chunker: walk spans in order, accumulate text until we cross
        // MAX_CHUNK_BYTES, then emit a chunk that spans [first, current].
        for chunk_text_and_range in chunk_spans(&page.spans, &span_ids) {
            let (start_id, end_id, text) = chunk_text_and_range;
            tx.execute(
                "INSERT INTO chunks (span_id_start, span_id_end, text) VALUES (?, ?, ?)",
                params![start_id, end_id, &text],
            )?;
            chunks.push(InsertedChunk {
                id: tx.last_insert_rowid(),
                text,
            });
        }
    }

    Ok((total_spans, chunks))
}

fn chunk_spans(spans: &[Span], span_ids: &[i64]) -> Vec<(i64, i64, String)> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut start_idx: Option<usize> = None;

    for (i, span) in spans.iter().enumerate() {
        // If adding this span would push us past the cap and we already have
        // content, flush.
        if !current.is_empty() && current.len() + span.text.len() > MAX_CHUNK_BYTES {
            if let Some(s) = start_idx {
                out.push((span_ids[s], span_ids[i - 1], std::mem::take(&mut current)));
            }
            start_idx = None;
        }
        if start_idx.is_none() {
            start_idx = Some(i);
        }
        current.push_str(&span.text);
    }
    if let Some(s) = start_idx {
        if !current.is_empty() {
            out.push((span_ids[s], *span_ids.last().unwrap(), current));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::pdf::Bbox;

    fn span(text: &str) -> Span {
        Span {
            start_offset: 0,
            end_offset: text.len(),
            text: text.to_string(),
            bbox: Bbox {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
            },
        }
    }

    #[test]
    fn chunker_keeps_short_pages_as_one_chunk() {
        let spans = vec![span("hello "), span("world")];
        let ids = vec![1_i64, 2];
        let chunks = chunk_spans(&spans, &ids);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, 1);
        assert_eq!(chunks[0].1, 2);
        assert_eq!(chunks[0].2, "hello world");
    }

    #[test]
    fn chunker_splits_at_byte_cap() {
        let long = "x".repeat(MAX_CHUNK_BYTES);
        let spans = vec![span(&long), span("tail")];
        let ids = vec![10_i64, 11];
        let chunks = chunk_spans(&spans, &ids);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].0, 10);
        assert_eq!(chunks[0].1, 10);
        assert_eq!(chunks[1].0, 11);
        assert_eq!(chunks[1].1, 11);
    }

    #[test]
    fn sha256_hex_is_lowercase_64_chars() {
        let h = sha256_hex(b"hello");
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
