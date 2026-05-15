//! `evidence-cli` library. The binary entry point is `src/main.rs`; this
//! module exists so the command implementations are reachable from
//! integration tests in `tests/`.

#![deny(unsafe_code)]

pub mod commands;
pub mod ollama;
