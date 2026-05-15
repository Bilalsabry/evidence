# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Changed
- Crate root: `forbid(unsafe_code)` → `deny(unsafe_code)`. The single
  `unsafe` block lives in `storage::vec::register`, where the `sqlite-vec`
  FFI entry point is registered as a SQLite auto-extension via a `Once`.

[Unreleased]: https://github.com/Bilalsabry/evidence/compare/HEAD...HEAD
