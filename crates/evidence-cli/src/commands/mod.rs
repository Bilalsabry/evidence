//! Pure command implementations. Each takes its dependencies as parameters
//! so tests can inject mocks; `main.rs` wires the real implementations.

pub mod ingest;
pub mod query;
