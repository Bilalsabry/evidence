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
}
