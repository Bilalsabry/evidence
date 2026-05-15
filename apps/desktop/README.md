# evidence-desktop

Tauri 2 shell around `evidence-core`. Status: v0.3.0 work-in-progress. Issue [#18](https://github.com/Bilalsabry/evidence/issues/18) shipped the foundation (Rust↔web bridge + smoke ping). Library view, PDF viewer, chat panel, and ingest progress land in [#19](https://github.com/Bilalsabry/evidence/issues/19), [#20](https://github.com/Bilalsabry/evidence/issues/20), and [#22](https://github.com/Bilalsabry/evidence/issues/22).

## Develop locally (macOS today, Linux/Windows once #signed-releases lands)

```sh
# One-time:
cargo install tauri-cli --version "^2.0"
npm --prefix apps/desktop/web install

# Run the app with hot-reloaded frontend:
cd apps/desktop
cargo tauri dev
```

The window appears immediately. Click **Ping core** → it calls the `app_version` Tauri command and renders the result. First-ever launch downloads the `bge-small-en-v1.5` model lazily on the first query (not on app start).

## What's wired up

| Tauri command | Status |
| --- | --- |
| `app_version() -> String` | ✅ smoke target for #18 |
| `ingest(path, title) -> IngestSummary` | ✅ callable; UI integration in #22 |
| `query(question, k, check_support) -> Result<Answer, String>` | ✅ callable; UI integration in #20 |

The `AppState` struct holds:
- `Mutex<Storage>` — opened eagerly at startup (cheap).
- `OllamaBackend` — internally thread-safe; talks to the default `http://127.0.0.1:11434`.

The embedder (`BgeSmall::shared()`) and the reranker (`BgeReranker` for `--check-support`) initialize lazily on first use so the window doesn't block waiting for a model download.

## Out of the default build graph

`evidence-desktop` is a Cargo workspace member but is excluded from `default-members` in the root `Cargo.toml`. That means `cargo build` / `cargo test` from the repo root skip it, which keeps the default loop fast and Linux-CI-friendly (Tauri pulls in `libwebkit2gtk-4.1-dev`-class system deps).

Explicit opt-in:

```sh
cargo build -p evidence-desktop       # from repo root
cargo run   -p evidence-desktop       # launches without hot reload
cargo tauri dev                       # from apps/desktop (with frontend hot reload)
```

CI runs `cargo {clippy,test} --workspace --exclude evidence-desktop`. A platform-aware desktop job will land alongside signed releases.

## Layout

```
apps/desktop/
├── Cargo.toml         # tauri 2, evidence-core, evidence-cli
├── build.rs           # tauri_build::build()
├── tauri.conf.json    # v2 schema
├── capabilities/
│   └── default.json   # core:default permission for the main window
├── icons/
│   └── icon.png       # placeholder; replace before a real release
├── src/
│   ├── main.rs        # binary entry; delegates to lib::run
│   └── lib.rs         # AppState + #[tauri::command] handlers
└── web/               # Vite + React + TS frontend
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        └── styles.css
```
