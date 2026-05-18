# evidence

[![ci](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml/badge.svg)](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/Bilalsabry/evidence?display_name=tag)](https://github.com/Bilalsabry/evidence/releases)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

**Auditable, local-first AI research assistant.** Drop a PDF in, ask a question, and every sentence in the answer is hyperlinked to the exact span on the exact page. If the model can't cite, it refuses to answer.

> Status: headless CLI, the public eval harness, the FDA-label benchmark, and the Tauri desktop app (PDF viewer + chat) are all shipped. Findings are supported on the controlled v2 injected FDA-label benchmark; they are not real-world prevalence rates.

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

## What works today

The validator is a three-gate decomposition, all three gates shipped and exercised by the eval harness:

1. **Existence** — the cited span ID resolves to a row in the database.
2. **In-context** — the cited span was in the prompt's chunk set for *this* query.
3. **Entailment** — a real NLI cross-encoder (`NliCrossEncoder`, `crates/evidence-core/src/query/nli.rs`) classifies `(cited span, sentence)` and the verdict must be `Supports`.

| Capability | Where | Status |
| --- | --- | --- |
| Headless ingest + hybrid (BM25 + vector) retrieval + span-level citations | `evidence-cli`, `evidence-core` | ✅ Shipped |
| NLI-backed support gate (`distilbert-base-uncased-mnli` default; DeBERTa-v3 selectable via `--nli-model`) | `evidence-core/src/query/nli.rs` | ✅ Shipped |
| Public eval harness — `run` / `inject` / `lint` / `stats` / `author` / `fetch` / `compare` / `metrics` / `audit-faithfulness` subcommands | `crates/evidence-eval` | ✅ Shipped |
| FDA drug-label benchmark (v2): 254 hand-shaped `valid` seeds → 1,270 matched-pair injected examples | `docs/paper/fda_label_bench_PROVENANCE.md`, `benchmark-results.md` | ✅ Shipped |
| Per-rule + composition results (§5.1–§5.4) | `docs/paper/section5-results.draft.md`, `rule-metrics.md`, `nli-comparison.md` | ✅ Shipped |
| Tauri desktop app — library view, PDF viewer, chat panel, ingest | `apps/desktop` | ✅ Shipped |

Headline results, supported on the controlled v2 injected FDA-label benchmark (not real-world prevalence):

- **Existence and in-context rules are exact** on the injected set: precision 1.000 / recall 1.000 for their owned classes (§5.2, `rule-metrics.md`). These two rules are model-independent.
- **The support rule recalls 0.992** of genuine support failures; its precision (0.873) is bounded by conservative NLI false-refusals, not a flaw in the decomposition (§5.2).
- **The three error classes are fully disjoint on the injected benchmark (100% by the operational blindness measure):** no failure is caught by a rule other than the one that owns it (§5.3, `docs/paper/CLAIMS.md`). The decomposition holds on the controlled benchmark.
- The composed validator improves should-refuse F1 by **+56.3 points** over the strongest *available* single-rule baseline (§5.4).

### Open / future work

These are genuinely not done and are not claimed:

- **Natural-failure study** (§5.6) — real LLM hallucinations with no injection, on a held-out corpus. Not yet run; this is the practical-prevalence claim and is explicitly out of scope of the current numbers.
- **ALCE comparison** (§5.5) — translating an ALCE subset into this framework. Not yet run.
- **Cross-domain** — the benchmark is single-domain (FDA labels). Generalization beyond it is unmeasured.
- **Clause-level decomposition / paraphrase tolerance** — splitting multi-claim sentences and a stronger checkpoint for near-paraphrases; noted as open work in [`docs/EVAL.md`](docs/EVAL.md).
- Signed cross-platform desktop releases and a docs site (`v1.0.0`).

## Architecture at a glance

| Layer | Crate | Highlights |
| --- | --- | --- |
| Ingest | `evidence-core` | `pdfium-render` for byte-accurate spans; SHA-256 dedupe; one-transaction write. |
| Storage | `evidence-core` | SQLite with `STRICT` tables, FTS5 (BM25), and `sqlite-vec` (`vec0`, 384-dim). Numbered migrations driven by `user_version`. |
| Retrieval | `evidence-core` | BM25 + `vec0` KNN, fused with Reciprocal Rank Fusion (`k_rrf = 60`). Natural-language input is sanitized into a safe FTS5 expression. |
| Embedding | `evidence-core` | `bge-small-en-v1.5` via `fastembed` (model auto-downloaded). |
| Query | `evidence-core` | `LlmBackend` trait + in-tree `testing::*Backend` impls. Validator rejects uncited / out-of-context / unentailed citations (real NLI cross-encoder for the entailment gate). |
| CLI | `evidence-cli` | `clap` v4. `OllamaBackend` over HTTP with `format: "json"`. |
| Eval | `evidence-eval` | Harness with `run`/`inject`/`lint`/`stats`/`author`/`fetch`/`compare`/`metrics`/`audit-faithfulness`. See [`docs/EVAL.md`](docs/EVAL.md). |
| Desktop | `apps/desktop` | Tauri 2 shell: library view, PDF viewer, chat panel, ingest. Excluded from default build graph; opt in with `cargo build -p evidence-desktop`. |

## Build

```sh
cargo build --workspace
cargo test --workspace --all-targets
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Security policy in [`SECURITY.md`](SECURITY.md).

## License

Apache-2.0. See [`LICENSE`](LICENSE).
