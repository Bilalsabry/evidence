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

use rusqlite::Connection;
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
