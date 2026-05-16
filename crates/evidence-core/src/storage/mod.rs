//! Persistent storage: SQLite plus `FTS5` (bundled with rusqlite) and the
//! `vec0` module from `sqlite-vec`, loaded on demand.
//!
//! [`Storage`] owns a single `rusqlite::Connection`. The schema is applied by
//! [`migrate`](migrations::migrate) on `open` and is idempotent — calling it
//! twice is a no-op.
//!
//! ```no_run
//! use evidence_core::storage::Storage;
//!
//! let storage = Storage::open_in_memory().unwrap();
//! storage.conn().execute_batch("SELECT count(*) FROM documents").unwrap();
//! ```

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod migrations;
pub mod vec;

/// Errors surfaced by the storage layer.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Wraps an underlying SQLite error from `rusqlite`.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    /// A migration step failed. The `version` is the SQL file's numeric
    /// prefix; `message` carries the underlying SQLite error text.
    #[error("migration {version} failed: {message}")]
    Migration { version: i32, message: String },
}

/// A SQLite-backed evidence store. Wraps a single connection; cheap to move,
/// not `Clone` because the underlying connection is not.
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open a file-backed store at `path`, creating it if missing. Applies
    /// any pending migrations before returning.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if the file cannot be opened or
    /// [`StorageError::Migration`] if a schema step fails.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        vec::ensure_registered();
        let conn = Connection::open(path)?;
        Self::prepare(conn)
    }

    /// Open an ephemeral in-memory store. Useful for tests and one-shot
    /// scripts.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if SQLite cannot allocate the
    /// connection (essentially never) or [`StorageError::Migration`] on
    /// schema failure.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        vec::ensure_registered();
        let conn = Connection::open_in_memory()?;
        Self::prepare(conn)
    }

    fn prepare(conn: Connection) -> Result<Self, StorageError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrations::migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Read-only access to the underlying connection.
    #[must_use]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutable access for transactions and prepared statements.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Read every document in the index, newest-first by `ingested_at`.
    /// `limit` and `offset` paginate; pass `limit = usize::MAX` to skip
    /// the cap.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] for query failures.
    pub fn list_documents(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DocumentInfo>, StorageError> {
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let offset_i64 = i64::try_from(offset).unwrap_or(0);
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, sha256, title, page_count, ingested_at \
             FROM documents \
             ORDER BY ingested_at DESC, id DESC \
             LIMIT ? OFFSET ?",
        )?;
        let rows = stmt
            .query_map([limit_i64, offset_i64], |row| {
                Ok(DocumentInfo {
                    id: row.get(0)?,
                    sha256: row.get(1)?,
                    title: row.get(2)?,
                    page_count: row.get(3)?,
                    ingested_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The absolute filesystem path a document was ingested from, if
    /// recorded. `Ok(None)` means either the document doesn't exist or it
    /// predates migration 004 (which added the column). The desktop PDF
    /// viewer uses this to re-read the original bytes.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] for query failures.
    pub fn document_source_path(&self, doc_id: i64) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT source_path FROM documents WHERE id = ?")?;
        let path = stmt
            .query_row([doc_id], |row| row.get::<_, Option<String>>(0))
            .optional()?
            .flatten();
        Ok(path)
    }

    /// Resolve a span to its document, 1-indexed page number, and
    /// bounding box (PDF points, origin bottom-left). `Ok(None)` if no
    /// span has that id. The desktop viewer uses this to scroll to and
    /// highlight a cited span.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] for query failures or if
    /// `bbox_json` is not valid `{x0,y0,x1,y1}` JSON (a corrupt index;
    /// surfaced rather than silently dropped).
    pub fn span_location(&self, span_id: i64) -> Result<Option<SpanLocation>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT p.doc_id, p.page_num, s.bbox_json \
             FROM spans s JOIN pages p ON p.id = s.page_id \
             WHERE s.id = ?",
        )?;
        let row = stmt
            .query_row([span_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .optional()?;
        let Some((doc_id, page_num, bbox_json)) = row else {
            return Ok(None);
        };
        let bbox: crate::ingest::pdf::Bbox = serde_json::from_str(&bbox_json).map_err(|e| {
            StorageError::Sqlite(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))
        })?;
        Ok(Some(SpanLocation {
            doc_id,
            page_num,
            bbox,
        }))
    }
}

/// Lightweight summary of a row in `documents`. Returned by
/// [`Storage::list_documents`]; carries enough to render a library
/// table without pulling page text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: i64,
    pub sha256: String,
    pub title: Option<String>,
    pub page_count: i64,
    pub ingested_at: i64,
}

/// Where a span lives: which document, which page, and the bounding box
/// (PDF points, origin bottom-left) the desktop viewer overlays a
/// highlight on. Returned by [`Storage::span_location`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpanLocation {
    pub doc_id: i64,
    /// 1-indexed page number.
    pub page_num: i64,
    pub bbox: crate::ingest::pdf::Bbox,
}
