//! Core ingestion, retrieval, and citation logic for evidence.
//!
//! `unsafe_code` is denied at the crate root; the only escape hatch is the
//! sqlite-vec auto-extension registration in [`storage::vec`], where FFI is
//! unavoidable and the call is documented and confined to a `Once`.

#![deny(unsafe_code)]

pub mod ingest;
pub mod retrieval;
pub mod storage;

/// Returns the current crate version.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
