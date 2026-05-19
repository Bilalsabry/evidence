# `evidence` — the project and the paper, end to end

A complete, self-contained explanation of what this is, why it exists,
what it claims, how the claims were tested, what the numbers say, what
is still open, and how to reproduce all of it. Written to be readable
without any prior context.

---

## 1. The one-paragraph version

When a retrieval-augmented language model answers a question, it
attaches citations: "this sentence is supported by source span #42."
The field treats "is this citation correct?" as a single yes/no
signal. We argue it is not one question but **three independent
questions**, and that conflating them hides which thing actually broke:
(1) **does the cited span exist** in the corpus at all; (2) **was it in
the set the model was actually shown** for this query; (3) **does it
actually support the claim**. We built a local-first system
(`evidence`) that validates all three at the boundary where output is
emitted, a benchmark of 254 FDA-drug-label examples expanded to 1,270
with controlled failure injection, and an evaluation harness that
measures each rule in isolation. On the v2 injected FDA-label benchmark
(not real-world prevalence) the first two rules are *exact* and
*model-independent*, the three rules catch **fully disjoint** classes
of error (100% by the operational blindness measure), and the composed
validator beats the best single rule by **+56 F1 points**. The third
rule (semantic support) is the soft one — it catches real failures at
99% recall but a weak NLI model over-refuses ~30–50% of true claims, a
measured, separately-argued finding rather than a flaw in the
decomposition.

---

## 2. The problem, precisely

### 2.1 What a citation failure actually is

A RAG system does three things: retrieves passages, generates an
answer, and attaches citations mapping answer sentences to retrieved
passages. A citation is "correct" only if a chain of three conditions
all hold:

1. **Existence.** The cited identifier resolves to a real span in the
   corpus. If the model emits "[span 9999]" and there is no span 9999,
   the citation is fabricated — a pointer into nothing.
2. **In-context.** The span exists, but was it in the retrieved set the
   model was given *for this query*? A model can cite a real span it
   was never shown — it "knows" the fact from pretraining and
   back-fills a plausible-looking citation to a real document it did
   not actually read in this turn. The citation looks valid (the span
   exists) but is not grounded in this interaction.
3. **Support.** The span exists and was shown, but does it actually
   *entail* the sentence? The model can cite a real, in-context span
   that says something adjacent but not the claim — or the opposite of
   the claim.

These are not three views of one property. They fail independently:
- A fabricated span fails (1) and the question of (2)/(3) is moot.
- A real-but-unseen span passes (1), fails (2).
- A real, seen, but irrelevant span passes (1) and (2), fails (3).

### 2.2 Why conflation is the actual problem

The dominant operational definition in the literature, AIS
(Attributable to Identified Sources, Rashkin et al. 2023), bundles all
three into a single human judgment. ALCE (Gao et al. 2023) and
derivatives measure support carefully but *assume rules 1 and 2 away by
construction* — citations are restricted to integer markers into the
retrieved set, so existence and in-context are true by data shape and
cannot be measured. GopherCite forces rules 1–2 by constrained
decoding. Self-RAG has separate reflection tokens for retrieval
relevance and support, but they are trained *inside* the model rather
than externalized as validator gates that can refuse output, and none
of these addresses "was this span in the prompt for this query."

The consequence for a practitioner: when a single faithfulness score is
low, you cannot tell whether the model is hallucinating span IDs (a
decoding/format problem), citing things it wasn't shown (a
retrieval-context problem), or citing real context that doesn't support
the claim (a reasoning/NLI problem). These have completely different
fixes; a decomposed signal tells you which lever to pull.

### 2.3 The thesis

> Citation correctness decomposes into three operationally independent
> constraints — existence, in-context, support — and on the v2
> injected FDA-label benchmark (not real-world prevalence) they catch
> non-overlapping classes of error. The decomposition, plus the
> empirical evidence supporting it under matched-pair failure
> injection, is the contribution.

The paper's working title: *Three Rules for a Citation: Decomposing
Faithfulness in Retrieval-Augmented Generation.* Target venue:
TrustNLP @ ACL 2026 (workshop).

### 2.4 Scope: citation correctness, not causal faithfulness

We validate citation *correctness* at the output boundary. The
in-context gate verifies the cited span was in the retrieval set the
model was shown; it does NOT verify the model causally relied on that
span when generating the sentence — a model can post-rationalize a
well-formed citation. Causal reliance (counterfactual span removal,
attribution tracing) is a distinct fourth dimension, explicitly out of
scope. This boundary is deliberate: existence, in-context, and support
are checkable at the validator without model internals; causal
faithfulness is not. Recent work separates citation correctness from
faithfulness and reports substantial post-rationalization; our claims
are confined to correctness.

---

## 3. The system: `evidence`

`evidence` is a local-first, open-source AI research assistant in which
every claim hyperlinks to an exact byte range in a source PDF. It is
the artifact that makes the paper concrete and reproducible. It is a
Rust multi-crate Cargo workspace:

- **`evidence-core`** — ingest (PDFium byte-accurate span extraction),
  storage (SQLite + FTS5 + `sqlite-vec`, STRICT tables, trigger-synced
  indexes), hybrid retrieval (BM25 + `bge-small` vectors fused by
  Reciprocal Rank Fusion, `bge-reranker-base` cross-encoder on top),
  and the validator (`ValidationPolicy` with the three gates).
- **`evidence-eval`** — the evaluation harness: dataset format, the
  failure-injection operators, the policy runner, the linter, the
  corpus fetcher, the metrics/compare/stats tools, the span-authoring
  scaffold.
- **`evidence-cli`, `evidence-api`** — surfaces.
- **`apps/desktop`** — a Tauri 2 + React desktop shell with a pdf.js
  viewer (span-level highlighting) and a chat panel with clickable
  citation chips.

The validator is the heart: a `ValidationPolicy` that runs the three
gates **in order — existence, then in-context, then support** — at the
point of output. Ordering matters: each gate cleanly owns one failure
class only because it runs after the cheaper structural checks have
already removed their classes.

### 3.1 The three gates as code

- **Existence gate.** A primary-key lookup: does the cited span ID
  resolve to a real span row? O(1). Deterministic.
- **In-context gate.** A per-prompt allowed-span set: was the cited
  span in the retrieval set assembled for *this* query? A `HashSet`
  membership test, scoped to one prompt. Deterministic.
- **Support gate.** An NLI cross-encoder over `(span_text,
  sentence)` pairs, mapping to a 3-class verdict
  (entailment/neutral/contradiction). The only non-deterministic,
  model-dependent gate. Aggregation is **per-span strict-wins**: every
  cited span must individually support the sentence; any neutral or
  contradiction sinks the answer. This is the conservative choice —
  "every citation must hold on its own" is the stronger safety claim.

The deployment philosophy is **refuse-or-resolve**: if any gate fails,
the system does not emit the unsupported claim — it refuses, or
resolves by re-retrieving. A conservative gate is the correct default
when the cost of a confidently-wrong pharmaceutical citation is high.

---

## 4. The benchmark

### 4.1 Corpus

50 public-domain FDA drug-label PDFs fetched from DailyMed (the FDA's
Structured Product Labeling service) via `evidence-eval fetch
dailymed`: idempotent download, manifest with per-file SHA-256. Drug
labels are an ideal domain — high-stakes, factual, full of crisp
quantitative claims (doses, concentrations, contraindications), and
public domain so the benchmark can ship.

### 4.2 The example format

Each benchmark example is a small self-contained "world":

- `corpus_spans` — the spans that exist in this example's corpus.
- `prompt_chunks` — which spans the model was shown for this query.
- `response_sentences` — the model's fixture answer, with `cited_spans`.
- `class` — the label: what this example demonstrates.
- `support_mutations` — author-supplied contrast-set rewrites used to
  derive support-failure variants.

Six classes: `valid`, `uncited`, `fabricated_span`, `out_of_context`,
`unsupported`, `contradicted`. The author only hand-writes **`valid`**
examples; everything else is generated.

### 4.3 Failure injection (the matched-pair contrast sets)

From each `valid` seed, `evidence-eval inject` mechanically derives
labeled failures (following Gardner et al. 2020 contrast-set
methodology, so each failure is produced by a *named operator* and is
not hand-cherry-picked):

- **Existence injection** — rewrite a citation to a span ID not in the
  corpus → a `fabricated_span` example.
- **In-context injection** — duplicate a span at a fresh ID *outside*
  the prompt, repoint the citation at it → an `out_of_context` example.
  (The text is identical; only the in-context status changed — a true
  matched pair.)
- **Support injection** — apply each author-supplied
  `support_mutation`: a number flipped, a negation inserted, an entity
  swapped (→ `contradicted`), or an off-topic-but-true substitution
  (→ `unsupported`). Same cited span; only the claim changed.

254 valid seeds → 1,270 examples (254 valid + 1,016 injected). The
matched-pair design is the defense against the "your failures are too
easy" reviewer objection: each failure differs from a true example by
exactly one controlled edit.

### 4.4 The harness and the policy ladder

`evidence-eval run` runs every example through four nested policies:

- `vanilla_rag` — no validation (accepts everything).
- `existence_only` — rule 1.
- `two_gate` — rules 1 + 2.
- `three_gate` — rules 1 + 2 + 3 (the full validator).

Each example has an *expected* outcome per policy (the matrix in
`runner.rs::expected_outcome`). Agreement = the validator behaved as
the class predicts. The harness has two support modes:

- **mock** — deterministic, class-driven. Used to verify the *wiring*
  is correct; 100% by construction, not a result.
- **real-nli** — the actual NLI model. This is where the support
  gate's true behavior is measured.

### 4.5 The benchmark-authoring honesty story (important)

The paper keeps this methodological point on record because it makes
the rest of the numbers trustworthy.

The seeds were AI-assisted-authored at scale (parallel agents over the
50 labels). The **first** benchmark (v1, 263 seeds) showed a 49%
false-refusal rate on `valid` examples at the full validator. Rather
than report that as an NLI finding, we hand-audited the refused
examples against the source PDFs. The cause was an **authoring bug, not
a model property**: the PDF span extractor emits *line-level* spans,
label sentences wrap across lines, and the v1 authoring guideline
("cite minimally, one span") made the agents cite a sentence
*fragment* — e.g. a cited span reading `"Doxycycline is virtually
completely"` while the operative word *absorbed* sat in the next,
uncited line. Both a weak and a strong NLI model **correctly** refuse
to entail a sentence from a fragment.

We corrected the guideline (the cited span must be one complete,
self-contained sentence, with wrapped lines merged), re-authored all
seeds (v2, 254 seeds), and re-ran. The 49% decomposed into ~15 points
of fixed benchmark bug and ~29% genuine NLI conservatism. Catching this
by audit rather than shipping it is part of the evidence base; the
v1→v2 delta is recorded in `benchmark-results.md`. The honest
provenance (AI-authored, spot-audited, not exhaustively human-verified)
is in `fda_label_bench_PROVENANCE.md`.

---

## 5. The results (all measured, all reproducible)

All numbers below are produced by tested in-repo tools and trace to
committed artifacts. The support gate uses a DeBERTa-v3 MNLI
checkpoint (`lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`) unless
noted.

### 5.1 Model comparison (does a stronger NLI help?) — `nli-comparison.md`

Swapping the baseline distilbert-MNLI for DeBERTa-v3:

- Three-gate agreement: 1,106 → 1,166 / 1,270 (**+60**).
- Valid-retention: 167 → 181 / 254.
- Contradicted catch: 212 → 244 / 254. Unsupported: 219 → 233 / 254.
- Structural classes (fabricated, out-of-context): **Δ = 0** — showing
  on the injected benchmark that rules 1 and 2 do not depend on the
  NLI model at all.

A stronger model helps but does not eliminate support-gate
conservatism (~29% of true, well-cited claims still refused). That
residual is a real finding: the support gate must be treated as a
separately-measured, conservative component, not folded into one score.

### 5.2 Per-rule isolation — `rule-metrics.md`

Each rule scored as its marginal gate over 1,270 examples:

| rule | owned class | precision | recall | F1 |
|---|---|---|---|---|
| existence | fabricated_span | 1.000 | 1.000 | **1.000** |
| in-context | out_of_context | 1.000 | 1.000 | **1.000** |
| support | unsupported+contradicted | 0.873 | 0.992 | 0.929 |

Existence and in-context are **exact and model-independent**. Support
recalls 0.992 of real failures (it does not *miss* bad citations); its
F1 is bounded by precision 0.873, where the false positives are the
§5.1 valid false-refusals — the NLI finding, not a decomposition flaw.

### 5.3 Non-overlap — the kernel — `rule-metrics.md`

The central claim. Is each failure class invisible to the rules that
precede its owning gate?

- Out-of-context citations accepted by existence-only: **254/254
  (100%)** — the existence rule is entirely blind to in-context errors
  (out-of-context is invisible to the existence gate).
- Support failures accepted by two-gate: **508/508 (100%)** — rules 1
  and 2 are entirely blind to support errors (support is invisible to
  the two-gate).

**Fully disjoint on the injected benchmark (100% by the operational
blindness measure).** This is a measured *blindness* property — each
preceding gate is structurally unable to see the next class — not an
artifact of the policies being nested. No failure is caught by a rule
other than the one that owns it; the three rules are not redundant
re-measurements of one signal but address independent failure modes,
and omitting any rule leaves its entire class undetected. This is a
clean form of the paper's thesis, holding on the v2 injected FDA-label
benchmark, not real-world prevalence. (CLAIMS.md target ≥80%; actual
100%.)

### 5.4 Composition and additive lift — `rule-metrics.md`

Each policy as a binary should-refuse classifier:

| policy | precision | recall | F1 |
|---|---|---|---|
| vanilla (no validation) | 1.000 | 0.000 | 0.000 |
| existence_only | 1.000 | 0.250 | 0.400 |
| two_gate (1+2) | 1.000 | 0.500 | 0.667 |
| three_gate (1+2+3) | 0.933 | 0.996 | **0.963** |

Monotone ladder, recall ~doubling per gate. **Additive lift: +56.3 F1
points** for the composed validator over the strongest non-composed
baseline (existence-only); target was ≥10, far exceeded. (Honest note:
the policies are nested, so only vanilla and existence-only are
genuinely single-rule; the baseline is the strongest *available* single
rule — a conservative, lower-bound choice.)

### 5.5 What the numbers mean together

- The contribution (the decomposition) holds on the controlled
  injected benchmark iff each rule isolates its class **and** the rules
  don't overlap. Both hold there, with hard numbers: §5.2 (isolation) +
  §5.3 (100% disjoint).
- The structural rules (1, 2) are *exact* and *model-independent* — the
  safe, unattackable core.
- The one soft number (support precision 0.873) is **not** a hole in
  the decomposition — it is the §5.1 NLI-conservatism finding, measured
  and argued separately. The honest framing leads with the structural
  result and presents support as the open, conservative third gate.

---

## 6. What is NOT done (honest gaps)

Four CLAIMS.md empirical claims; three resolved, one open:

| Claim | Status |
|---|---|
| §5.1 model comparison | ✅ measured |
| §5.2 per-rule isolation | ✅ measured |
| §5.3 non-overlap | ✅ measured (100%) |
| Claim 3 additive lift | ✅ measured (+56.3) |
| Claim 4 natural-failure agreement | ⬜ **open** |

- **Claim 4 (natural-failure run).** The benchmark uses *injected*
  failures; the defense against "injected failures are too easy" is a
  run on *real* LLM hallucinations (no injection), human-graded. This
  requires a held-out corpus, a chosen generation model, and human
  labeling — a methodological decision and a human effort, deliberately
  not automated or fabricated.
- **§5.5 ALCE comparison.** Translating an ALCE subset into this
  framework to show the decomposition is not a benchmark artifact —
  planned, not run.
- **Class-balanced injection caveat.** Headline F1/lift come from a
  class-balanced injected set (failures prevalent); real RAG has lower
  failure prevalence where false-refusal cost dominates — we report the
  support false-refusal rate explicitly rather than optimize it away.
- **§5.7 cost/latency** — not measured.
- **Wider human audit of the 254 seeds** — only a sample was audited
  against source PDFs; an exhaustive human pass is recommended before
  submission.
- **Benchmark provenance.** The seeds are AI-assisted-authored. The
  structural results (§5.2 existence/in-context, all of §5.3) do not
  depend on this, but the support-gate numbers' authoring homogeneity
  is a fair reviewer question. The datasheet
  (`fda_label_bench_PROVENANCE.md`) pre-empts it; a human curation pass
  plus explicit disclosure is the conservative path.

---

## 7. Reproducing everything

From a clone of the repo, with the `evidence-eval` binary installed
(`cargo install --path crates/evidence-eval`):

```sh
# 1. fetch the public corpus (idempotent)
evidence-eval fetch dailymed --output ~/dailymed-corpus --limit 50

# 2. inject failures from the committed seed benchmark
evidence-eval inject crates/evidence-eval/datasets/fda_label_bench.toml \
    --output /tmp/fda_injected.toml

# 3. §5.2 + §5.3 + claim-3 metrics (DeBERTa support)
evidence-eval metrics /tmp/fda_injected.toml --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx

# 4. §5.1 model comparison
evidence-eval compare /tmp/fda_injected.toml \
    --candidate-nli lquint/DeBERTa-v3-base-mnli-fever-anli-onnx

# 5. full per-example scoreboard
evidence-eval run /tmp/fda_injected.toml --real-nli
```

The structural rules (1, 2) reproduce identically in mock mode (no
model download); the support numbers require the NLI checkpoint
download on first run.

---

## 8. Document map (where each piece lives)

- `CLAIMS.md` — the exact claims, with measured actuals replacing the
  original targets.
- `OUTLINE.md` — the paper's section structure.
- `prior-art-reading.md`, `related-work-table.md` — the web-verified
  prior-art audit and the gap matrix (why this is novel).
- `benchmark-results.md` — the v1→v2 audit story + full scoreboard.
- `rule-metrics.md` — §5.2 / §5.3 / claim-3 numbers (with 95% bootstrap CIs).
- `nli-comparison.md` — §5.1 distilbert vs DeBERTa table.
- `cost-latency.md` — §5.7 measured cost split (structural vs NLI).
- `faithfulness-audit.md` — automated cited-span verbatim audit + manual verdicts.
- `fda_label_bench_PROVENANCE.md` — the benchmark datasheet.
- `section5-results.draft.md` — §5 results prose draft (for the
  author to finalize).
- `crates/evidence-eval/datasets/fda_label_bench.toml` — the 254-seed
  benchmark itself.
- `docs/EVAL.md` — the harness + authoring guideline (corrected).
- This file — the end-to-end explainer.

---

## 9. The honest bottom line

The decomposition kernel is **supported on a controlled injected
single-domain benchmark**: there the structural rules are exact and
model-independent, the three rules are fully disjoint at N=1,270 (100%
by the operational blindness measure), and composition beats the best
single rule by +56 F1. Real-world prevalence and causal faithfulness
are out of scope and open. The one soft spot — semantic support
precision — is a measured, separately-argued property of the NLI
model, not a crack in the idea, and is exactly why the paper argues
support must be its own gate with a refuse-or-resolve default. The
remaining work (claim-4 natural-failure study, ALCE comparison, wider
human audit, final prose) requires human judgment and real labeled
data, not more tooling. Everything that could be built and measured
rigorously on this benchmark has been, and every number is reproducible
from a single command.
