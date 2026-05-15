# evidence

[![ci](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml/badge.svg)](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

**Auditable, local-first AI research assistant.** Drop a PDF in, ask a question, and every sentence in the answer is hyperlinked to the exact span on the exact page. If the model can't cite, it refuses to answer.

> Status: pre-alpha. Headless CLI end-to-end is the `v0.1.0` target.

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
| `v0.1.0` | Headless ingest + hybrid retrieval + naive citations (CLI) | Ready |
| `v0.2.0` | Cross-encoder reranker + citation validator | Planned |
| `v0.3.0` | Tauri desktop UI with PDF viewer + citation chips | Planned |
| `v0.5.0` | Public eval harness + benchmarks | Planned |
| `v1.0.0` | Signed cross-platform releases, docs site | Planned |

## Build

```sh
cargo build --workspace
cargo test --workspace
```

## License

Apache-2.0. See [LICENSE](LICENSE).
