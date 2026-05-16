//! Idempotent schema migrations driven by SQLite's `user_version` pragma.
//!
//! Each entry in [`MIGRATIONS`] is `(version, sql)`. Versions strictly
//! increase. On [`migrate`], every entry with `version > current_user_version`
//! is applied in a transaction and the pragma is bumped to that version.
//! Calling [`migrate`] twice on the same database is a no-op.

use rusqlite::Connection;

use super::StorageError;

const MIGRATIONS: &[(i32, &str)] = &[
    (1, include_str!("migrations/001_initial.sql")),
    (2, include_str!("migrations/002_chunks_indexes.sql")),
    (3, include_str!("migrations/003_chunks_fts_triggers.sql")),
    (4, include_str!("migrations/004_documents_source_path.sql")),
];

/// Apply every migration newer than the connection's current `user_version`.
///
/// # Errors
///
/// Returns [`StorageError::Migration`] if any step's `execute_batch` fails,
/// preserving the underlying SQLite message.
pub fn migrate(conn: &Connection) -> Result<(), StorageError> {
    let mut current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for &(version, sql) in MIGRATIONS {
        if version <= current {
            continue;
        }
        conn.execute_batch(sql)
            .map_err(|e| StorageError::Migration {
                version,
                message: e.to_string(),
            })?;
        conn.pragma_update(None, "user_version", version)?;
        current = version;
    }
    Ok(())
}

/// Latest schema version. Useful for assertions in tests.
#[must_use]
pub fn latest_version() -> i32 {
    MIGRATIONS.last().map_or(0, |(v, _)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_strictly_increase() {
        let mut last = 0;
        for &(v, _) in MIGRATIONS {
            assert!(
                v > last,
                "migration versions must strictly increase, got {v} after {last}",
            );
            last = v;
        }
    }
}
