//! Core ingestion, retrieval, and citation logic for evidence.

#![forbid(unsafe_code)]

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
