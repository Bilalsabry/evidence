# evidence — design

## Problem

LLMs assemble plausible-sounding text from learned statistics. They have no inherent mechanism for separating what they know from what they made up. In high-stakes domains — pharma, legal, medical, finance — plausible-but-wrong is worse than "I don't know," because plausible-but-wrong gets cited as fact.

Retrieval-augmented generation helps, but most implementations cite at the **document** level (e.g., "see PDF X"). That isn't verifiable: the reader still has to read the whole document to confirm a single claim. The gap between "the model said X with a footnote" and "X is true on page 7, paragraph 3" is where hallucinations hide.

`evidence` closes that gap by enforcing **span-level citations**: every sentence in the answer is tied to a specific byte range on a specific page of a specific document. If the model cannot produce a valid citation, the system refuses to answer instead of guessing.

## Non-goals

- A general chat interface. `evidence` answers questions grounded in user-provided documents.
- A cloud service. The desktop app runs entirely on the user's machine. Cloud inference is opt-in.
- A document storage product. Documents stay on the user's disk; the index is a local SQLite database.

## High-level architecture

```
                    ┌──────────────────────────────┐
                    │       Tauri 2 desktop        │
                    │   (Rust core + web UI)       │
                    └──────────────┬───────────────┘
                                   │ local HTTP / IPC
                                   ▼
              ┌────────────────────────────────────────┐
              │            evidence-api                │
              │ axum router: documents · query · span  │
              └──────────────┬─────────────────────────┘
                             │
                             ▼
              ┌────────────────────────────────────────┐
              │            evidence-core               │
              │ ingest │ chunk │ embed │ retrieve │    │
              │ rerank │ generate │ validate           │
              └──────────────┬─────────────────────────┘
                             │
              ┌──────────────┼──────────────────────────┐
              ▼              ▼                          ▼
        ┌──────────┐   ┌──────────┐              ┌─────────────┐
        │ SQLite + │   │ SQLite + │              │ Ollama /    │
        │ FTS5     │   │ vec0     │              │ AI Gateway  │
        │ (BM25)   │   │ (vectors)│              │ (inference) │
        └──────────┘   └──────────┘              └─────────────┘
```

## Crate layout

| Crate | Responsibility |
| --- | --- |
| `evidence-core` | Ingestion, retrieval, citation logic. No I/O beyond SQLite and the inference backend. |
| `evidence-api` | `axum` HTTP server that the desktop UI and CLI talk to. |
| `evidence-cli` | Headless interface for scripting and CI evaluation. |

## Data model

- **Document** — the file the user gave us, identified by SHA-256.
- **Page** — a single page of a document with raw extracted text.
- **Span** — a contiguous byte range on a page, with bounding-box metadata for UI highlighting. Spans are the only thing citations point to.
- **Chunk** — a retrieval unit: one or more contiguous spans on the same page, sized for the embedder's context window.

Chunks are the unit of retrieval; spans are the unit of citation. Keeping them separate lets the system index at one granularity and cite at another.

## Citation protocol

1. The retriever returns a ranked list of chunks (BM25 + vector, score-fused).
2. The reranker rescores the top-N with a cross-encoder.
3. The generator receives the top-k chunks and is prompted to return a structured response: a list of sentences, each tagged with one or more `span_id` references.
4. The citation validator walks the response. For each sentence:
   - Every cited span must exist in the database.
   - Every cited span must appear in the chunks shown to the model. (Prevents the model from referencing real spans it didn't actually consult.)
   - The cited spans must lexically support the claim. (Initial implementation: reranker score as a proxy. A stronger NLI-based check is a `v0.3` goal.)
5. If validation fails, the system re-prompts up to N times. If it still fails, the system returns a refusal.

The refusal is the feature, not a bug.

## Why local-first

- **Trust.** Documents in this domain are confidential by default. A pharma reviewer cannot send a draft FDA submission to a third-party cloud.
- **Auditability.** The user can inspect every component — the index, the prompts, the model — without trusting a vendor.
- **Cost.** A single user with a few thousand pages costs zero per query after the model is downloaded.

## Why SQLite, not a dedicated vector database

- One file, one process, zero ops. The index lives in the OS application-support directory and is portable.
- `sqlite-vec` provides ANN search; `FTS5` provides BM25. Hybrid retrieval needs both, and they share a single transaction boundary — the index stays consistent.
- Expected workload is 10k–1M chunks per user. SQLite handles this comfortably.
- If a future workload demands a dedicated vector store, the retrieval layer is the natural seam to swap.

## Open questions (tracked as issues)

- **Embedder.** `bge-small-en-v1.5` is the leading default. Open question: domain-specific embedder for biomedical text?
- **Chunking.** Paragraph-aware vs. sentence-window with overlap. Initial implementation: paragraph-aware with overlap.
- **Validator strength.** Reranker score proxy vs. NLI-based entailment. Initial: proxy; NLI in `v0.3`.
- **Refusal threshold.** How aggressive should the validator be? Initial: refuse on any unresolved cite; tune from eval data.
