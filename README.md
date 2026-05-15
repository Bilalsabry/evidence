# evidence

[![ci](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml/badge.svg)](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/Bilalsabry/evidence?display_name=tag)](https://github.com/Bilalsabry/evidence/releases)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

**Auditable, local-first AI research assistant.** Drop a PDF in, ask a question, and every sentence in the answer is hyperlinked to the exact span on the exact page. If the model can't cite, it refuses to answer.

> Status: `v0.1.0` shipped. Headless CLI is feature-complete; desktop UI is the `v0.3.0` milestone.

## Quickstart

```sh
# Install (from a checkout — pre-built binaries are a v1.0 deliverable).
cargo install --path crates/evidence-cli

# One-time: bring up a local LLM. Any model with strong JSON adherence works.
ollama serve &
ollama pull llama3.1:8b-instruct

# Ingest a PDF and ask it a question.
evidence --db evidence.db ingest paper.pdf --title "Trial Protocol v1"
evidence --db evidence.db query "what is the contraindication?"
```

Example output:

```
The contraindication list excludes pregnant women and patients under 18. [p7:span42]
```

If the model produces a sentence the validator can't trace to a real span, `evidence` exits with status `2` and prints a refusal:

```
I can't answer that without a verifiable citation.
Reason: model cited span 999, which was not in the chunks shown
```

## Why

LLMs hallucinate. In pharma, legal, medical, and finance work, plausible-but-wrong is worse than "I don't know" — because plausible-but-wrong gets quoted as fact. Most retrieval-augmented tools cite at the **document** level, which still requires reading the whole document to verify a single claim. `evidence` enforces **span-level** citations: every sentence resolves to a specific byte range on a specific page. If the model cannot produce a valid citation, the system returns a refusal instead of an answer.

The refusal is the feature.

## How it works

```
PDF → page → spans → chunks → (BM25 + vector) retrieval → rerank
                                                              │
                                                              ▼
                                           constrained generation w/ citation slots
                                                              │
                                                              ▼
                                                   citation validator
                                                  (valid? → answer; else → refuse)
```

Full design: [`docs/DESIGN.md`](docs/DESIGN.md).

## Roadmap

| Milestone | Scope | Status |
| --- | --- | --- |
| [`v0.1.0`](https://github.com/Bilalsabry/evidence/releases/tag/v0.1.0) | Headless ingest + hybrid retrieval + naive citations (CLI) | ✅ Shipped |
| `v0.2.0` | Cross-encoder reranker + NLI-based citation lexical-support check | Planned |
| `v0.3.0` | Tauri desktop UI with PDF viewer + citation chips | Planned |
| `v0.5.0` | Public eval harness + benchmarks | Planned |
| `v1.0.0` | Signed cross-platform releases, docs site | Planned |

## Architecture at a glance

| Layer | Crate | Highlights |
| --- | --- | --- |
| Ingest | `evidence-core` | `pdfium-render` for byte-accurate spans; SHA-256 dedupe; one-transaction write. |
| Storage | `evidence-core` | SQLite with `STRICT` tables, FTS5 (BM25), and `sqlite-vec` (`vec0`, 384-dim). Numbered migrations driven by `user_version`. |
| Retrieval | `evidence-core` | BM25 + `vec0` KNN, fused with Reciprocal Rank Fusion (`k_rrf = 60`). Natural-language input is sanitized into a safe FTS5 expression. |
| Embedding | `evidence-core` | `bge-small-en-v1.5` via `fastembed` (model auto-downloaded). |
| Query | `evidence-core` | `LlmBackend` trait + in-tree `testing::*Backend` impls. Validator rejects uncited / out-of-context citations. |
| CLI | `evidence-cli` | `clap` v4. `OllamaBackend` over HTTP with `format: "json"`. |

## Build

```sh
cargo build --workspace
cargo test --workspace --all-targets
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Security policy in [`SECURITY.md`](SECURITY.md).

## License

Apache-2.0. See [`LICENSE`](LICENSE).
