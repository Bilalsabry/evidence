//! Tauri command handlers and process-wide state for the evidence
//! desktop app.
//!
//! Storage is opened eagerly at startup (cheap); the embedder downloads
//! its model on first query via [`BgeSmall::shared`] so the window
//! appears immediately on first launch.
//!
//! The reranker used by the support check is also lazy — it's only
//! initialized when a query passes `check_support = true`.

#![deny(unsafe_code)]

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use evidence_cli::ollama::{OllamaBackend, DEFAULT_BASE_URL, DEFAULT_MODEL};
use evidence_core::ingest::{ingest_pdf, IngestSummary};
use evidence_core::query::{answer_query, Answer, RerankerSupportChecker, SupportChecker};
use evidence_core::retrieval::{BgeReranker, BgeSmall};
use evidence_core::storage::Storage;

/// Default DB path inside the OS's app-data directory. Tauri provides a
/// helper for this in the runtime (`app.path()`), but for v0.3 a fixed
/// project-relative path keeps the smoke path dependency-free.
const DEFAULT_DB_FILENAME: &str = "evidence.db";

/// Process-wide state managed by Tauri. `Storage` is `Mutex`-wrapped because
/// `rusqlite::Connection` is not `Sync`; the Ollama backend is internally
/// thread-safe.
pub struct AppState {
    storage: Mutex<Storage>,
    llm: OllamaBackend,
}

impl AppState {
    fn new(db_path: PathBuf) -> Result<Self, String> {
        let storage = Storage::open(&db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            storage: Mutex::new(storage),
            llm: OllamaBackend::new(DEFAULT_BASE_URL, DEFAULT_MODEL),
        })
    }
}

/// Sanity ping: returns the core crate's version. This is the smoke target
/// for #18 — the frontend calls it on mount to prove the IPC wiring works.
#[tauri::command]
fn app_version() -> String {
    evidence_core::version().to_string()
}

#[tauri::command]
fn ingest(
    state: tauri::State<AppState>,
    path: String,
    title: Option<String>,
) -> Result<IngestSummary, String> {
    let embedder = BgeSmall::shared().map_err(|e| e.to_string())?;
    let mut storage = state
        .storage
        .lock()
        .map_err(|e| format!("storage mutex poisoned: {e}"))?;
    ingest_pdf(&mut storage, embedder, path, title.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn query(
    state: tauri::State<AppState>,
    question: String,
    k: usize,
    check_support: bool,
) -> Result<Answer, String> {
    let embedder = BgeSmall::shared().map_err(|e| e.to_string())?;
    let storage = state
        .storage
        .lock()
        .map_err(|e| format!("storage mutex poisoned: {e}"))?;

    let support_checker: Option<Box<dyn SupportChecker>> = if check_support {
        let reranker = shared_reranker().map_err(|e| e.to_string())?;
        Some(Box::new(RerankerSupportChecker::new(reranker)))
    } else {
        None
    };

    answer_query(
        &storage,
        embedder,
        &state.llm,
        &question,
        k,
        support_checker.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Process-wide reranker. Initialized once per process on the first
/// `check_support = true` query; subsequent calls reuse it.
fn shared_reranker() -> Result<&'static BgeReranker, String> {
    static RERANKER: OnceLock<Result<BgeReranker, String>> = OnceLock::new();
    RERANKER
        .get_or_init(|| BgeReranker::new().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(Clone::clone)
}

/// Entry point. Called by `main.rs`.
///
/// # Panics
///
/// Panics if `Storage::open` fails or if Tauri fails to start — both
/// indicate a corrupt local install and there's nothing useful to do
/// beyond surface the error to the user.
pub fn run() {
    let db_path = PathBuf::from(DEFAULT_DB_FILENAME);
    let state = AppState::new(db_path).expect("opening evidence index");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![app_version, ingest, query])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_matches_core() {
        assert_eq!(app_version(), evidence_core::version());
    }
}
