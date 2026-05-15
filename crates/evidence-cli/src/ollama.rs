//! Ollama backend for [`LlmBackend`].
//!
//! Talks to a locally-running Ollama server over HTTP. Ollama supports a
//! `format: "json"` parameter that constrains decoding to valid JSON,
//! which is the closest thing to structured output the local-model
//! ecosystem offers today. We pair that with a prompt that explicitly
//! requests the [`RawAnswer`] schema.
//!
//! Ollama isn't tested in CI (no server available) — `tests/smoke.rs` uses
//! the deterministic `MockBackend` from `evidence_core::query::testing`.
//! Unit tests here cover the prompt-building logic.

use evidence_core::query::{LlmBackend, LlmError, Prompt, RawAnswer};
use serde::Deserialize;

/// Default Ollama HTTP endpoint, matching the upstream defaults.
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434";

/// Default model. Picked for solid JSON-output adherence at a manageable
/// memory footprint. Override via [`OllamaBackend::with_model`].
pub const DEFAULT_MODEL: &str = "llama3.1:8b-instruct";

/// HTTP client for an Ollama server.
pub struct OllamaBackend {
    base_url: String,
    model: String,
    agent: ureq::Agent,
}

impl OllamaBackend {
    /// Backend pointing at the default local Ollama on the default model.
    #[must_use]
    pub fn local_default() -> Self {
        Self::new(DEFAULT_BASE_URL, DEFAULT_MODEL)
    }

    /// Backend pointing at `base_url` with the given `model`.
    #[must_use]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            agent: ureq::AgentBuilder::new().build(),
        }
    }

    /// Swap the model on an existing backend (chainable).
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl LlmBackend for OllamaBackend {
    fn answer(&self, prompt: &Prompt) -> Result<RawAnswer, LlmError> {
        let body = serde_json::json!({
            "model": self.model,
            "stream": false,
            "format": "json",
            "prompt": build_prompt_text(prompt),
        });

        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let response = self
            .agent
            .post(&url)
            .send_json(body)
            .map_err(|e| LlmError::Transport(e.to_string()))?;

        let payload: GenerateResponse = response
            .into_json()
            .map_err(|e| LlmError::Transport(e.to_string()))?;

        let raw: RawAnswer = serde_json::from_str(&payload.response)
            .map_err(|e| LlmError::Malformed(e.to_string()))?;
        Ok(raw)
    }
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

/// Build the user prompt sent to Ollama. The prompt is plain text; the
/// `format: "json"` parameter constrains decoding.
pub fn build_prompt_text(prompt: &Prompt) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str(
        "You are evidence, a research assistant that answers strictly from supplied context.\n",
    );
    s.push_str("Return a single JSON object matching this schema:\n");
    s.push_str("{ \"sentences\": [ { \"text\": string, \"span_ids\": [integer, ...] } ] }\n");
    s.push_str("Rules:\n");
    s.push_str("- Every sentence must include at least one span_id from the context below.\n");
    s.push_str("- Only use span_ids that appear in the context. Do not invent.\n");
    s.push_str(
        "- If you cannot answer with supported citations, return an empty sentences array.\n\n",
    );
    s.push_str("Context chunks:\n");
    for chunk in &prompt.chunks {
        s.push_str(&format!(
            "- chunk_id={}, span_ids={}..{}: {}\n",
            chunk.chunk_id, chunk.span_id_start, chunk.span_id_end, chunk.text,
        ));
    }
    s.push_str("\nQuestion: ");
    s.push_str(&prompt.question);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use evidence_core::query::ChunkContext;

    fn prompt_with_one_chunk() -> Prompt {
        Prompt {
            question: "what is the contraindication?".to_string(),
            chunks: vec![ChunkContext {
                chunk_id: 7,
                text: "The contraindication is pregnancy.".to_string(),
                span_id_start: 3,
                span_id_end: 4,
            }],
        }
    }

    #[test]
    fn prompt_text_lists_chunks_and_question() {
        let text = build_prompt_text(&prompt_with_one_chunk());
        assert!(text.contains("chunk_id=7"));
        assert!(text.contains("span_ids=3..4"));
        assert!(text.contains("The contraindication is pregnancy."));
        assert!(text.contains("what is the contraindication?"));
    }

    #[test]
    fn prompt_text_documents_the_no_citation_refusal_path() {
        let text = build_prompt_text(&prompt_with_one_chunk());
        assert!(text.contains("return an empty sentences array"));
    }
}
