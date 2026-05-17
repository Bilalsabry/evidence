# Evaluating Closed-Loop Citation

This document covers the evaluation harness shipped in
[`crates/evidence-eval/`](../crates/evidence-eval/) and how it underpins
the in-progress technical post *Closed-Loop Citation: A Three-Gate
Validator for LLM Hallucinations in Regulated Workflows*.

## What we're measuring

A standard RAG system shows the model some chunks, the model produces
an answer with citations, and the citations are taken at face value.
The model can — and does — produce citations that look right but
don't ground the claim. `evidence` enforces three gates over every
sentence:

1. **Existence.** The cited span ID resolves to a row in the database.
2. **In-context.** The cited span was in the prompt's chunk set for
   *this* query. The model cannot reach into the rest of the corpus.
3. **Entailment.** An NLI cross-encoder classifies (cited span,
   sentence) and the verdict is `Supports`.

This document is about measuring those gates: how does each one perform
on a labeled set of hallucinations, and how much does each gate add to
catch rate?

## The harness

[`evidence-eval`](../crates/evidence-eval/) loads a TOML dataset, builds
an in-memory SQLite store per example, runs the example's fixture
response through every [`ValidationPolicy`](../crates/evidence-core/src/query/mod.rs),
and produces a markdown report.

### Dataset format

Every example declares the world it lives in: a `corpus_spans` list
seeds the in-memory store; `prompt_chunks` declares the subset the
model was shown; `response_sentences` is the fixture model output.
The `class` tag names the hallucination the example demonstrates.

```toml
[[example]]
name = "contradicted_enrollment_number"
class = "contradicted"
description = "Canonical hallucination: cited span says 240, model says 1000."
corpus_spans = [
    { id = 110, page = 1, text = "The trial enrolled 240 patients with stage III NSCLC." },
]
prompt_chunks = [
    { id = 100, span_range = [110, 110], text = "The trial enrolled 240 patients with stage III NSCLC." },
]
response_sentences = [
    { text = "The trial enrolled 1000 patients.", cited_spans = [110] },
]
```

### Hallucination classes

| Class | What the example shows | First gate to refuse |
|---|---|---|
| `valid` | Well-formed, in-context, entailed sentence. | — (all gates accept) |
| `uncited` | Sentence with empty `cited_spans`. | Shape (`Uncited`) |
| `fabricated_span` | Cited span ID isn't in the DB. | Gate 1 (`UnknownSpan`) |
| `out_of_context` | Cited span exists but wasn't in the prompt. | Gate 2 (`OutOfContext`) |
| `unsupported` | Cited span is off-topic. NLI verdict: `Neutral`. | Gate 3 (`Unsupported`) |
| `contradicted` | Cited span contradicts the sentence. NLI verdict: `Contradicts`. | Gate 3 (`Contradicted`) |

### Validator order

The validator runs the gates in this order per cited span:

1. **Existence.** DB lookup. Fails with `UnknownSpan` if missing.
2. **In-context.** HashSet lookup against the prompt's allowed-span
   set. Fails with `OutOfContext` if missing.
3. **Entailment.** Support check (only after all spans for the
   sentence pass gates 1 and 2). Fails with `Unsupported` or
   `Contradicted` depending on the NLI verdict.

We do existence before in-context so each gate cleanly catches its own
hallucination class — a fabricated ID surfaces as `UnknownSpan` even
when it would also fail in-context. The harness's catch matrix is
deterministic as a result.

## Bootstrap results

The bootstrap dataset has 12 examples, two per class. Running it
produces this catch matrix:

| class \ policy | `vanilla_rag` | `existence_only` | `two_gate` | `three_gate` |
|---|---|---|---|---|
| `valid` | 2 / 0 | 2 / 0 | 2 / 0 | 2 / 0 |
| `uncited` | 2 / 0 | 0 / 2 | 0 / 2 | 0 / 2 |
| `fabricated_span` | 2 / 0 | 0 / 2 | 0 / 2 | 0 / 2 |
| `out_of_context` | 2 / 0 | 2 / 0 | 0 / 2 | 0 / 2 |
| `unsupported` | 2 / 0 | 2 / 0 | 2 / 0 | 0 / 2 |
| `contradicted` | 2 / 0 | 2 / 0 | 2 / 0 | 0 / 2 |

Each cell is `accepted / refused`. The first refusing policy moves
left-to-right per row — each gate adds catch rate for its own class.

**Caveat.** The bootstrap harness uses class-derived mock support
checkers (`SupportingChecker`, `NeutralChecker`, `ContradictingChecker`)
so the test stays deterministic. It measures the validator wiring, not
the support model's accuracy. The real model's accuracy is measured
separately under `--real-nli` (see below).

## Real-NLI run

Pass `--real-nli` to swap the class-derived mock for
`NliCrossEncoder::shared()` (the same instance the CLI uses,
`Xenova/distilbert-base-uncased-mnli`). The agreement rate then
measures **the NLI model's accuracy on our labeled examples**.

On the 30-example bootstrap:

| class | mock-mode agreement | real-NLI agreement | divergences |
|---|---|---|---|
| `valid` | 5 / 5 | 3 / 5 | NLI refused 2 valid sentences |
| `uncited` | 5 / 5 | 5 / 5 | — (gate runs before NLI) |
| `fabricated_span` | 5 / 5 | 5 / 5 | — (gate runs before NLI) |
| `out_of_context` | 5 / 5 | 5 / 5 | — (gate runs before NLI) |
| `unsupported` | 5 / 5 | 5 / 5 | NLI correctly Neutral on every off-topic citation |
| `contradicted` | 5 / 5 | 5 / 5 | NLI correctly Contradicts every numeric / negation / temporal flip |

**Real-NLI overall agreement: 118 / 120 (98.3%).**

The two divergences:

- `valid_two_spans_one_sentence` — sentence "The intent-to-treat
  population included all randomized patients, with median follow-up
  of 18.4 months." cites two spans, each supporting half the claim.
  The NLI cross-encoder scores each `(span, sentence)` pair
  independently; neither span entails the full composite sentence, so
  the strict-wins aggregation returns `Neutral`.
- `valid_chunk_with_span_range` — sentence wording is a near-paraphrase
  of the cited span ("Grade 3+ adverse events" vs. "Grade 3+ events").
  The NLI model is conservative on the rewording.

Both failure modes are real cases the paper will note as **open work**:

- **Clause-level decomposition.** Split a multi-claim sentence into
  atomic sub-claims, NLI each, and aggregate. Most-cited paper:
  Min et al. (2023) *FActScore*.
- **Paraphrase tolerance.** A stronger / larger NLI checkpoint, or
  fine-tuning the cross-encoder on domain-specific paraphrase pairs,
  closes most of the gap. Cheap, deferred until we have eval data
  from real customer corpora to calibrate against.

## Running the harness

```sh
# Run the bootstrap dataset, print report to stdout
cargo run -p evidence-eval -- crates/evidence-eval/datasets/bootstrap.toml

# Save to file
cargo run -p evidence-eval -- crates/evidence-eval/datasets/bootstrap.toml --output report.md

# Run the harness as a test (CI-friendly)
cargo test -p evidence-eval
```

Exit code `0` means every row agrees with the expected matrix; exit
code `2` means a row diverged (regression in the validator).

## Failure injection: matched-pair contrast sets

Beyond hand-authored class-labeled examples, the harness ships an
**injection pipeline** that derives labeled failure variants from
verified-good seeds. Each `valid` example yields:

- One **existence** variant — citation rewritten to a span ID not in the
  corpus.
- One **in-context** variant — a duplicate-text span added at a fresh
  ID outside the prompt; citation rewritten to point at it.
- One **support** variant per `support_mutations` entry (author-supplied
  sentence rewrite, labeled `unsupported` or `contradicted`).

The implementation lives at
[`crates/evidence-eval/src/inject.rs`](../crates/evidence-eval/src/inject.rs).
Construction details are intentionally explicit so the paper can cite
each operator by name, following Gardner et al. 2020 (contrast sets).

### Running the pipeline

```sh
# Fetch a real corpus (FDA drug labels) for the paper-grade benchmark:
cargo run -p evidence-eval -- fetch dailymed --limit 50 --output ./dailymed-corpus

# Generate matched-pair variants from a dataset of valid seeds:
cargo run -p evidence-eval -- inject \
    crates/evidence-eval/datasets/bootstrap.toml \
    --output /tmp/bootstrap-injected.toml

# Run the augmented dataset through every policy:
cargo run -p evidence-eval -- run /tmp/bootstrap-injected.toml
```

The `fetch dailymed` step walks the public DailyMed v2 API, downloads
PDFs idempotently, and writes a `manifest.toml` with stable set IDs,
SHA-256 hashes, and fetch timestamps. Reruns skip files already on
disk that match the manifest.

On the current bootstrap (5 valid seeds with author-supplied support
mutations) the pipeline emits 14 matched-pair variants, expanding the
total dataset to 44 examples. Validator agreement remains 100% across
all 176 (example, policy) rows.

The injection pipeline replaces the Week 4 task in the paper plan with
a now-standing artifact: the larger dataset is built by hand-authoring
~300 valid seeds with support mutations, then running `evidence-eval
inject` to materialize the failure-injection corpus.

## Authoring the benchmark

Install the CLI once, then loop per label:

```sh
cd <repo> && cargo install --path crates/evidence-eval --force

evidence-eval fetch dailymed --output ~/dailymed-corpus --limit 50
evidence-eval author --pdf ~/dailymed-corpus/labels/<file>.pdf \
    --name <slug> --page 1 > ~/evidence-bench/<slug>.toml
# …edit the TOML: pick spans, write the question + grounded answer…
evidence-eval lint  ~/evidence-bench/        # file OR directory
evidence-eval stats ~/evidence-bench/        # class balance as you go
evidence-eval inject ~/evidence-bench/<slug>.toml --output /tmp/inj.toml
evidence-eval run   /tmp/inj.toml --real-nli
```

Spans are **line-level** (one readable run per span), not per-glyph —
paste 2–5 lines from the comment block into `corpus_spans`.

### Authoring guidelines (learned from the first real label)

1. **Cite minimally.** A `valid` example's `cited_spans` must contain
   only spans that *individually* entail the claim. The support gate is
   per-span strict-wins (`query/nli.rs`): it checks each cited span
   independently and **any single `Neutral` refuses the whole answer**.
   Adding a section-header or context span (neutral w.r.t. the claim)
   is structurally guaranteed to turn a true answer into
   `refused(unsupported)`. More citations is not safer here.

2. **Phrase `valid` answers lexically close to the cited span.** A
   terse span (`"Zinc Oxide 20%"`) will not entail an elaborated
   sentence (*"the active ingredient … at a concentration of 20%"*)
   under a weak NLI model. This does **not** mean distort the claim to
   please the model: the benchmark's structural results (§5.2/§5.3) are
   measured under the deterministic *mock* checker (always 100%);
   `--real-nli` valid-retention is the *separate NLI-quality column*,
   where weak-model false-refusals are expected data, not authoring
   errors. Author the truthful claim; tight phrasing just keeps the
   real-NLI column readable.

3. **Aggregation is a fixed design choice, not a knob to author
   around.** Per-span strict-wins is intentional — "every citation must
   hold on its own" is the stronger safety claim. A
   concatenated-evidence alternative (join cited spans, check once)
   would suit union-supported claims; it is noted as future work, not
   shipped. Author to strict-wins.

### Worked `valid` template

A clean single-span quantitative example. Note: one tight cited span,
answer hugging the span text, two contrast-set mutations for `inject`.

```toml
[[example]]
name = "spf30_active_ingredient_concentration"
class = "valid"
description = "Single-span quantitative claim from an OTC sunscreen label."

corpus_spans = [
    { id = 10, page = 1, text = "Active Ingredients" },
    { id = 20, page = 1, text = "Zinc Oxide 20%" },
]
prompt_chunks = [
    { id = 1, span_range = [10, 20], text = "Active Ingredients Zinc Oxide 20%" },
]
response_sentences = [
    { text = "The active ingredient is zinc oxide 20%.", cited_spans = [20] },
]

[[example.support_mutations]]
text = "The active ingredient is zinc oxide 10%."   # number perturbation
class = "contradicted"

[[example.support_mutations]]
text = "Reapply at least every two hours."          # topic substitution
class = "unsupported"
```

Run it through `lint → inject → run --real-nli`. The structural rows
(existence / in-context / contradicted / unsupported) must all be ✓;
the `Valid | three_gate` row under `--real-nli` may still be ✗ with
`distilbert-MNLI` — that is the measured weak-model column, and it is
the concrete §5.1 motivation for DeBERTa-v3-large, not a bug.

## Open work, toward the paper

This PR ships the harness, the bootstrap dataset, and the determinism
guarantee. The paper needs three further pieces:

1. **Expand the bootstrap dataset** to ~100 examples, drawn from real
   pharma / clinical-trial / FDA-label text (preserving annotation
   provenance). Manually-labeled cases per class.
2. **Real-model run.** Replace the class-driven mock with
   `NliCrossEncoder`, run the same dataset, and report per-class
   accuracy of the NLI model. This is the *actual* catch rate of the
   support gate.
3. **Real-LLM run.** Replace the fixture model response with calls to
   a local Ollama model (and an `AI Gateway`-backed remote model).
   Measure: how often do real LLMs produce each hallucination class
   on a held-out pharma corpus? This is the *practical* claim — that
   `evidence` materially reduces hallucination in real workflows.

Each of these is a follow-up issue, scoped after this harness lands.

## References

- Cormack, Clarke, Buettcher (2009). *Reciprocal Rank Fusion outperforms
  Condorcet and individual rank learning methods.* (For the hybrid
  retriever, not the validator.)
- Honovich et al. (2022). *TRUE: Re-evaluating Factual Consistency
  Evaluation.*
- Min et al. (2023). *FActScore: Fine-grained Atomic Evaluation of
  Factual Precision.*
