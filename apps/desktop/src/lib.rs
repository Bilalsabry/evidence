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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use evidence_cli::ollama::{OllamaBackend, DEFAULT_BASE_URL, DEFAULT_MODEL};
use evidence_core::ingest::{
    ingest_pdf, ingest_pdf_with_progress, IngestProgress, IngestStage, IngestSummary,
};
use evidence_core::query::{answer_query, Answer, RerankerSupportChecker, SupportChecker};
use evidence_core::retrieval::{BgeReranker, BgeSmall};
use evidence_core::storage::{DocumentInfo, Storage};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

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
    /// Sequence used by `ingest_with_progress` to label each ingest in
    /// emitted events. The frontend filters its progress listener by id.
    next_ingest_id: AtomicU64,
}

impl AppState {
    fn new(db_path: PathBuf) -> Result<Self, String> {
        let storage = Storage::open(&db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            storage: Mutex::new(storage),
            llm: OllamaBackend::new(DEFAULT_BASE_URL, DEFAULT_MODEL),
            next_ingest_id: AtomicU64::new(1),
        })
    }
}

/// Payload of the `ingest:progress` Tauri event.
#[derive(Debug, Serialize, Clone)]
struct IngestProgressEvent {
    /// Matches the value `ingest_with_progress` returns.
    ingest_id: u64,
    /// Stage tag — one of `hashing`, `parsing`, `persisting`, `embedding`, `done`.
    stage: &'static str,
    current: usize,
    total: usize,
}

fn stage_label(stage: IngestStage) -> &'static str {
    match stage {
        IngestStage::Hashing => "hashing",
        IngestStage::Parsing => "parsing",
        IngestStage::Persisting => "persisting",
        IngestStage::Embedding => "embedding",
        IngestStage::Done => "done",
    }
}

/// `IngestProgress` impl that emits `ingest:progress` events through the
/// Tauri runtime. Cheap to clone; cheap to call from the ingest thread.
struct TauriIngestProgress {
    app: AppHandle,
    ingest_id: u64,
}

impl IngestProgress for TauriIngestProgress {
    fn step(&self, stage: IngestStage, current: usize, total: usize) {
        // Emit errors are non-fatal — the ingest should still complete
        // even if the UI has gone away.
        let _ = self.app.emit(
            "ingest:progress",
            IngestProgressEvent {
                ingest_id: self.ingest_id,
                stage: stage_label(stage),
                current,
                total,
            },
        );
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

/// Returned payload from `ingest_with_progress`. The ingest_id matches
/// the `ingest:progress` event stream.
#[derive(Debug, Serialize)]
struct IngestStarted {
    ingest_id: u64,
    summary: IngestSummary,
}

/// Like [`ingest`] but emits `ingest:progress` events as each stage
/// completes. The frontend subscribes via `listen('ingest:progress', ...)`
/// and filters by `ingest_id`. Runs synchronously on the Tauri invoke
/// pool — the UI thread stays responsive because invokes are async on
/// the JS side.
#[tauri::command]
fn ingest_with_progress(
    app: AppHandle,
    state: tauri::State<AppState>,
    path: String,
    title: Option<String>,
) -> Result<IngestStarted, String> {
    let ingest_id = state.next_ingest_id.fetch_add(1, Ordering::SeqCst);
    let progress = TauriIngestProgress {
        app: app.clone(),
        ingest_id,
    };
    let embedder = BgeSmall::shared().map_err(|e| e.to_string())?;
    let mut storage = state
        .storage
        .lock()
        .map_err(|e| format!("storage mutex poisoned: {e}"))?;
    let summary = ingest_pdf_with_progress(
        &mut storage,
        embedder,
        path,
        title.as_deref(),
        Some(&progress),
    )
    .map_err(|e| e.to_string())?;
    Ok(IngestStarted { ingest_id, summary })
}

/// List all ingested documents, newest-first. Returns up to 200 rows;
/// pagination follow-up once the library view needs it.
#[tauri::command]
fn list_documents(state: tauri::State<AppState>) -> Result<Vec<DocumentInfo>, String> {
    let storage = state
        .storage
        .lock()
        .map_err(|e| format!("storage mutex poisoned: {e}"))?;
    storage.list_documents(200, 0).map_err(|e| e.to_string())
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
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_version,
            ingest,
            ingest_with_progress,
            list_documents,
            query
        ])
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
