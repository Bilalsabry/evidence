//! `evidence query <text>`: thin wrapper around
//! [`evidence_core::query::answer_query`] plus a stdout formatter.

use evidence_core::query::{answer_query, Answer, LlmBackend, QueryError};
use evidence_core::retrieval::Embedder;
use evidence_core::storage::Storage;

/// Run the query command end-to-end. Returns the validated [`Answer`].
///
/// # Errors
///
/// Returns [`QueryError`] for retrieval, backend, or validation failures.
pub fn run(
    storage: &Storage,
    embedder: &dyn Embedder,
    llm: &dyn LlmBackend,
    question: &str,
    k: usize,
) -> Result<Answer, QueryError> {
    answer_query(storage, embedder, llm, question, k)
}

/// Render an [`Answer`] in the human-readable CLI format. One sentence per
/// line, with citations as `[p<page>:span<id>]` footnote markers.
#[must_use]
pub fn format_answer(answer: &Answer) -> String {
    let mut out = String::new();
    for sentence in &answer.sentences {
        out.push_str(&sentence.text);
        for citation in &sentence.citations {
            out.push(' ');
            out.push_str(&format!(
                "[p{}:span{}]",
                citation.page_num, citation.span_id
            ));
        }
        out.push('\n');
    }
    out
}

/// Render a [`QueryError`] in the human-readable refusal format. Used by
/// the CLI when the validator rejects the model's output.
#[must_use]
pub fn format_refusal(err: &QueryError) -> String {
    format!("I can't answer that without a verifiable citation.\nReason: {err}\n")
}
