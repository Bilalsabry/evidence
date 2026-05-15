//! Integration tests for `OllamaBackend` against a local stub HTTP server.
//!
//! Gated on the `ollama-integration-tests` cargo feature so the default
//! `cargo test` run stays fast and the stub-server-only logic doesn't
//! leak into shipped binaries. Run with:
//!
//! ```sh
//! cargo test -p evidence-cli --features ollama-integration-tests --test ollama_integration
//! ```
//!
//! The stub speaks the subset of Ollama's `POST /api/generate` API that
//! [`OllamaBackend`] actually calls. Each test gets its own server on an
//! OS-assigned port; the server thread exits cleanly when the `Arc` drops.

#![cfg(feature = "ollama-integration-tests")]

use std::sync::{atomic::AtomicUsize, atomic::Ordering, Arc};
use std::thread;
use std::time::Duration;

use evidence_cli::ollama::OllamaBackend;
use evidence_core::query::{ChunkContext, LlmBackend, LlmError, Prompt};
use tiny_http::{Header, Response, Server};

/// Spin up a one-shot HTTP stub on an OS-assigned port and return its
/// base URL together with a counter of requests served. The server runs
/// until `Arc::strong_count` of the returned handle drops to 1.
fn start_stub(
    handler: impl Fn(tiny_http::Request) + Send + Sync + 'static,
) -> (String, Arc<AtomicUsize>) {
    let server = Server::http("127.0.0.1:0").expect("bind stub server");
    let port = server.server_addr().to_ip().expect("ip addr").port();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_thread = counter.clone();
    thread::spawn(move || {
        for request in server.incoming_requests() {
            counter_for_thread.fetch_add(1, Ordering::SeqCst);
            handler(request);
        }
    });
    // Tiny sleep to make sure the listener is in the kernel's accept queue
    // before any test makes a request — eliminates flakiness on slow CI.
    thread::sleep(Duration::from_millis(50));
    (format!("http://127.0.0.1:{port}"), counter)
}

fn prompt() -> Prompt {
    Prompt {
        question: "what is the contraindication?".to_string(),
        chunks: vec![ChunkContext {
            chunk_id: 1,
            text: "The contraindication is pregnancy.".to_string(),
            span_id_start: 1,
            span_id_end: 1,
        }],
    }
}

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
}

#[test]
fn happy_path_deserializes_to_raw_answer() {
    let (url, calls) = start_stub(|req| {
        let body = serde_json::json!({
            "response": "{\"sentences\":[{\"text\":\"The contraindication is pregnancy.\",\"span_ids\":[1]}]}"
        })
        .to_string();
        let response = Response::from_string(body).with_header(json_header());
        req.respond(response).unwrap();
    });

    let backend = OllamaBackend::new(url, "stub-model");
    let raw = backend
        .answer(&prompt())
        .expect("happy path should succeed");

    assert_eq!(raw.sentences.len(), 1);
    assert_eq!(raw.sentences[0].text, "The contraindication is pregnancy.");
    assert_eq!(raw.sentences[0].span_ids, vec![1]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn malformed_inner_json_returns_malformed_error() {
    let (url, _) = start_stub(|req| {
        let body = serde_json::json!({
            "response": "this is not json"
        })
        .to_string();
        let response = Response::from_string(body).with_header(json_header());
        req.respond(response).unwrap();
    });

    let backend = OllamaBackend::new(url, "stub-model");
    let err = backend
        .answer(&prompt())
        .expect_err("must error on bad inner JSON");
    assert!(
        matches!(err, LlmError::Malformed(_)),
        "expected Malformed, got {err:?}",
    );
}

#[test]
fn server_error_surfaces_as_transport_error() {
    let (url, _) = start_stub(|req| {
        let response = Response::from_string("server boom")
            .with_status_code(500)
            .with_header(json_header());
        req.respond(response).unwrap();
    });

    let backend = OllamaBackend::new(url, "stub-model");
    let err = backend.answer(&prompt()).expect_err("must error on 5xx");
    assert!(
        matches!(err, LlmError::Transport(_)),
        "expected Transport, got {err:?}",
    );
}

#[test]
fn empty_sentences_is_a_valid_raw_answer() {
    // Ollama returning `{"response": "{\"sentences\": []}"}` means the model
    // refused to answer. The backend itself should still deserialize this
    // successfully — refusal handling is the validator's job, not the
    // backend's.
    let (url, _) = start_stub(|req| {
        let body = serde_json::json!({
            "response": "{\"sentences\":[]}"
        })
        .to_string();
        let response = Response::from_string(body).with_header(json_header());
        req.respond(response).unwrap();
    });

    let backend = OllamaBackend::new(url, "stub-model");
    let raw = backend
        .answer(&prompt())
        .expect("empty-sentences response is valid JSON, must deserialize");
    assert!(raw.sentences.is_empty());
}

#[test]
fn malformed_outer_envelope_returns_transport_error() {
    // The outer envelope doesn't even include a `response` field.
    let (url, _) = start_stub(|req| {
        let body = "{\"unexpected\": true}".to_string();
        let response = Response::from_string(body).with_header(json_header());
        req.respond(response).unwrap();
    });

    let backend = OllamaBackend::new(url, "stub-model");
    let err = backend
        .answer(&prompt())
        .expect_err("must error on bad envelope");
    // ureq surfaces the deserialization-of-the-envelope failure through
    // `into_json`, which the backend wraps as Transport.
    assert!(
        matches!(err, LlmError::Transport(_)),
        "expected Transport for missing `response` field, got {err:?}",
    );
}
