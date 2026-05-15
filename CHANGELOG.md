# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `query::NliCrossEncoder` — real NLI cross-encoder via direct `ort` +
  `tokenizers` + `hf-hub` integration. Default model is
  `Xenova/distilbert-base-uncased-mnli` (~265 MB, auto-downloaded on
  first construction); label order is read from the model's
  `config.json` so substituting a different repo is a one-arg change.
  Inference runs behind a `Mutex<Session>` to keep the trait `Sync`.
- `query::NliSupportChecker` — `SupportChecker` implementation that
  classifies each cited span individually and aggregates with
  "strict-wins": any `Contradicts` wins, else any `Neutral` wins,
  else `Supports`.
- `query::SupportVerdict` is now 3-state (`Supports` / `Neutral` /
  `Contradicts`), matching textbook NLI labels. The v0.2
  `RerankerSupportChecker` keeps its 2-state behavior and never
  returns `Contradicts` (the relevance-as-proxy semantic doesn't have
  a contradiction signal).
- `QueryError::Contradicted` variant for the new refusal path.
- `query::testing::ContradictingSupport` mock and a matching smoke
  test.
- CLI `--support-mode rerank|nli` flag on `evidence query`. Default
  `nli` when `--check-support` is set (#21).
- `apps/desktop` — Tauri 2 shell over `evidence-core`. Vite + React + TS
  frontend, three Tauri commands (`app_version`, `ingest`, `query`), a
  default capability scoped to the main window, and a smoke screen that
  pings the core's version. Excluded from `default-members` so the
  default `cargo build`/`cargo test` and CI both skip it; explicit
  opt-in via `cargo run -p evidence-desktop` or `cargo tauri dev` from
  `apps/desktop/`.
- `IngestSummary` now derives `Serialize` + `Deserialize` so it
  round-trips through the Tauri IPC layer (#18).
- `retrieval::Reranker` trait + `BgeReranker` (`bge-reranker-base` via
  `fastembed`, model auto-downloads on first construction).
- `retrieval::hybrid_search_with_reranker` runs hybrid retrieval, then
  rescores the top `k * RERANK_CANDIDATE_FACTOR` (= 4) candidates with a
  cross-encoder and truncates to `k`. Returned `ChunkHit::score` is the
  reranker score, matching the larger-is-better contract.
- `HybridError::Rerank` variant wrapping `RerankError`.
- CI cache key updated to cover both models under `.fastembed_cache/` (#12).
- `query::SupportChecker` trait + `RerankerSupportChecker` shipping
  implementation that uses a cross-encoder as a proxy for entailment.
  `answer_query` takes an `Option<&dyn SupportChecker>`; `None` keeps
  v0.1.0 behavior. New `QueryError::Unsupported` variant for the
  refusal path. CLI: `--check-support` flag opts in (downloads
  `bge-reranker-base` on first use). `query::testing::{ApprovingSupport,
  RejectingSupport}` mocks (#13).

### Changed
- `answer_query` signature gained a trailing `support` parameter.
  Update callers to pass `None` to preserve v0.1.0 behavior.

### Tests
- `evidence-cli` gains an `ollama-integration-tests` cargo feature. When
  enabled, `tests/ollama_integration.rs` runs five scenarios against a
  local `tiny_http` stub server (happy path, malformed inner JSON,
  HTTP 5xx, empty `sentences` array, malformed outer envelope). CI runs
  this as a dedicated `test-ollama` job; the default `cargo test
  --workspace` still skips it (#14).

## [0.1.0] — 2026-05-15

### Added
- Cargo workspace skeleton (`evidence-core`, `evidence-api`, `evidence-cli`).
- Design document and high-level architecture.
- CI workflow: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
- `ingest::pdf::extract` — PDFium-backed PDF parser that emits 1-indexed
  `Page` records, each carrying `raw_text` and a contiguous, non-overlapping
  sequence of byte-offset `Span`s with PDF-coordinate bounding boxes. Typed
  `PdfError` distinguishes load failures, per-page failures, encrypted PDFs,
  and PDFium backend init failures. PDFium binary is auto-downloaded and
  cached via `pdfium-auto`; first-call download is serialized through a
  `OnceLock` so parallel callers don't race the cache (#1).
- `storage::Storage` — SQLite-backed store with `bundled` rusqlite (FTS5
  included) and runtime registration of `sqlite-vec` as an auto-extension.
  Schema lives in numbered SQL files under
  `crates/evidence-core/src/storage/migrations/`; the runner uses SQLite's
  `user_version` pragma and is idempotent. Initial schema: `documents`,
  `pages`, `spans`, `chunks` (all `STRICT`), plus `chunks_fts` (FTS5,
  content-linked) and `chunks_vec` (`vec0`, 384-dim) virtual tables (#2).
- `retrieval::bm25_search` — BM25 query over `chunks_fts`, returning ranked
  `ChunkHit` records with sign-flipped (higher-is-better) scores and the
  underlying span range. Migration 003 installs three triggers that keep
  `chunks_fts` in lockstep with `chunks` on INSERT/UPDATE/DELETE (#3).
- `retrieval::Embedder` trait + `BgeSmall` implementation (`fastembed`'s
  `bge-small-en-v1.5`, 384 dims, model auto-downloaded on first use).
- `retrieval::vector_search` and `retrieval::upsert_chunk_embedding` —
  KNN over `chunks_vec` using vec0's `k = ?` constraint; upsert deletes
  the prior row before insert because vec0 doesn't support `ON CONFLICT`.
- `retrieval::hybrid_search` + `rrf` — Reciprocal Rank Fusion combiner
  (default `k_rrf = 60`) over BM25 and vector hits, producing a single
  ranked `Vec<ChunkHit>`. Empty / whitespace queries short-circuit and
  skip the embedder (#4).
- `ingest::ingest_pdf` — end-to-end pipeline that parses a PDF, writes
  pages/spans/chunks/embeddings in one transaction, and refuses to
  re-ingest a document with a matching SHA-256.
- `query::answer_query` — full QA path: hybrid retrieve, prompt the LLM,
  validate citations, resolve each citation to `{ span_id, doc_id,
  page_num, start_offset, end_offset }`. Refuses any sentence without a
  citation, or with a citation outside the chunks shown.
- `query::LlmBackend` trait + `query::testing::{MockBackend,
  RefusingBackend, UncitedBackend, OutOfContextBackend}`.
- `evidence` CLI with `ingest <pdf>` and `query <text>` subcommands.
  `query` exits with status `2` if the validator refuses.
- `evidence_cli::ollama::OllamaBackend` — HTTP backend talking to a
  local Ollama server. Uses `format: "json"` plus a strict prompt that
  documents the no-citation refusal path (#5).

### Changed
- Crate root: `forbid(unsafe_code)` → `deny(unsafe_code)`. The single
  `unsafe` block lives in `storage::vec::register`, where the `sqlite-vec`
  FFI entry point is registered as a SQLite auto-extension via a `Once`.

[Unreleased]: https://github.com/Bilalsabry/evidence/compare/HEAD...HEAD
