//! Closed-Loop Citation evaluation harness.
//!
//! Each [`Example`] in a TOML dataset declares a small "world" — a corpus
//! of spans, a prompt (the subset shown to the model), and a fixture
//! model response. The example is tagged with a [`HallucinationClass`]
//! that names *what* the example demonstrates. The harness runs the
//! example through each [`Policy`] and asserts the validator behaves as
//! the class predicts.
//!
//! This isolates the validator from retrieval and from real-LLM
//! variance: each gate's catch rate per class is a fact about the
//! validator alone. A follow-on study (separate crate / paper section)
//! measures how often real LLMs produce each class in practice; that's
//! a separate question.

#![deny(unsafe_code)]

pub mod author;
pub mod dataset;
pub mod fetch;
pub mod inject;
pub mod lint;
pub mod report;
pub mod runner;

pub use author::{render_for_pdf, render_template, AuthorError, AuthorOptions};
pub use dataset::{load_dataset, Dataset, Example, HallucinationClass, SupportMutation};
pub use fetch::{fetch_dailymed, FetchConfig, FetchError, HttpClient, Manifest, UreqClient};
pub use inject::{
    inject_all_variants, inject_existence, inject_in_context, inject_support, InjectionConfig,
    InjectionError,
};
pub use lint::{has_errors, lint, render_report, Diagnostic, Severity};
pub use report::{write_markdown, Report};
pub use runner::{run, run_with_mode, Outcome, Policy, RunResult, SupportMode};
