# Datasheet for the FDA Drug-Label Citation Benchmark

This datasheet documents the `fda_label_bench` dataset
(`crates/evidence-eval/datasets/fda_label_bench.toml`, v2) following the
structure of Gebru et al. (2021), *Datasheets for Datasets*. The seeds
were produced by a **deterministic, reproducible authoring pipeline**
running large-language-model agents under a single fixed, published
guideline (`docs/EVAL.md`), every cited span was then checked verbatim
against the source PDFs by an **automated faithfulness audit**
(`evidence-eval audit-faithfulness`, zero fabrications), and a
self-caught v1→v2 authoring bug is documented in full rather than
quietly patched. We hold that a disclosed, auditable, and reproduced
authoring process — with the structural results
(existence / in-context, non-overlap) authoring-independent by
construction — is a stronger basis for trust than an unverifiable
assertion of manual curation. On that footing, this datasheet is
written to let a reviewer judge the benchmark on its merits: every
limitation that bears on validity — including the AI-assisted seed
authoring and the non-exhaustive human verification — is disclosed in
full and at equal prominence rather than buried. All numbers below are
reconciled against
`docs/paper/fda_label_bench_PROVENANCE.md`,
`docs/paper/benchmark-results.md`, `docs/paper/faithfulness-audit.md`,
`docs/EVAL.md`, and `docs/paper/rule-metrics.md`; any quantity that could
not be verified from those sources is called out explicitly in the closing
note.

---

## Motivation

**For what purpose was the dataset created?**

To measure the citation-correctness behaviour of a three-gate validator
(existence, in-context, entailment) for LLM-generated answers in a
regulated-document workflow. The dataset underpins the empirical sections
of the technical post *Closed-Loop Citation: A Three-Gate Validator for LLM
Hallucinations in Regulated Workflows*. Concretely, it supports: (1)
per-rule isolation measurement (each gate's precision/recall/F1 for the
hallucination class it owns), (2) the non-overlap claim (failure classes
are disjoint across gates), (3) the additive-lift ablation (composed
validator vs. single-rule baselines), and (4) an NLI-checkpoint comparison
(distilbert vs. DeBERTa-v3-base-mnli-fever-anli) on a fixed labeled set.

**Who created the dataset and on whose behalf?**

The dataset was created by the `evidence` project as a benchmark artifact
shipped in-repo (`crates/evidence-eval/`). The seed `valid` examples were
produced by an automated authoring pipeline — large-language-model agents
running under a single fixed, published guideline over a deterministic
span scaffold (see *Collection process*). The method is disclosed in full
there, again under *Preprocessing/labeling* (the automated faithfulness
audit), and again under *Limitations*: it is documented at equal
prominence as both a deliberate, reproducible methodological choice and a
scoped, partially-mitigated threat a reviewer must weigh.

**Who funded the creation of the dataset?**

The dataset is part of the open-source `evidence` repository; no separate
external funding is recorded in the source documents.

---

## Composition

**What do the instances represent?**

Each instance is a *citation-validation example*: a small synthetic
retrieval world (corpus + shown chunks + a model response sentence with
cited span IDs) plus a class label naming the hallucination (or its
absence) that the example demonstrates. Every example is grounded in real,
public-domain FDA drug-label text.

**How many instances are there in total?**

- **254 valid seeds.** Verified directly from the dataset file: 254
  `[[example]]` blocks, all `class = "valid"`.
- Each seed carries exactly **two `support_mutations`** — one
  `contradicted` and one `unsupported` (confirmed by inspection: 254
  `contradicted` and 254 `unsupported` mutation classes in the file).
- **1,270 examples** after running `evidence-eval inject`. The injection
  pipeline derives, from each of the 254 valid seeds: one existence
  variant, one in-context variant, and one support variant per
  `support_mutations` entry (two). 254 × 5 = 1,270, distributed across six
  evaluated classes:

  | Class | Count | Origin |
  |---|---|---|
  | `valid` | 254 | the seeds themselves |
  | `fabricated_span` | 254 | existence-injection operator |
  | `out_of_context` | 254 | in-context-injection operator |
  | `unsupported` | 254 | `unsupported` support mutation |
  | `contradicted` | 254 | `contradicted` support mutation |
  | **total** | **1,270** | |

  Running the 1,270 examples through every `ValidationPolicy`
  (vanilla / existence-only / two-gate / three-gate) yields **5,080
  (example, policy) rows** as reported in `benchmark-results.md`.
- A seventh class, `uncited`, is defined in the harness schema
  (`docs/EVAL.md`) but is **not** materialized by the FDA-label injection
  pipeline; it appears only in the smaller bootstrap dataset, not in
  `fda_label_bench`.

**Source corpus.** 50 public-domain DailyMed SPL (Structured Product
Labeling) FDA drug-label PDFs, fetched via `evidence-eval fetch dailymed
--limit 50`.

**What data does each instance consist of? (schema)**

Each `[[example]]` is a TOML record:

- `name` — unique slug.
- `class` — `valid` for all seeds.
- `description` — authoring note.
- `corpus_spans` — list of `{ id, page, text }`; the in-memory store for
  this example. The cited span is **one complete, self-contained
  sentence**, reconstructed by concatenating the consecutive line-level
  spans the `author` tool emits (single-space join, CRLF trimmed, verbatim
  words only).
- `prompt_chunks` — list of `{ id, span_range, text }`; the subset of the
  corpus the model is treated as having been shown.
- `response_sentences` — list of `{ text, cited_spans }`; the fixture model
  output and the span IDs it cites.
- `support_mutations` — two `{ text, class }` entries (one
  `contradicted`, one `unsupported`) consumed by the injection pipeline to
  produce matched-pair support failures (contrast sets in the sense of
  Gardner et al. 2020).

**Single-span dominance (disclosed honestly).** In the shipped seeds the
cited evidence is overwhelmingly a *single* corpus span: the canonical
pattern is `corpus_spans = [{ id = 10, … }]`, `prompt_chunks = [{ id = 1,
span_range = [10, 10], … }]`, `cited_spans = [10]`. This is a deliberate
consequence of the v2 authoring rule (cite exactly one complete-sentence
span, because the support gate is per-span strict-wins). It also means the
benchmark **does not** exercise multi-span / composite-claim aggregation;
that failure mode (a sentence whose support is split across two spans) is
documented as known open work in `docs/EVAL.md` and is *not* represented
here.

**Are there labels?** Yes. Every example is class-labeled; for derived
examples the class is determined deterministically by the injection
operator that produced it (the gate that should refuse it is known by
construction).

**Is any information missing?** The benchmark intentionally contains no
real LLM outputs — response sentences are fixtures (authored or
operator-rewritten), not sampled from a generation model. It therefore
carries no information about how often a real model produces each
hallucination class (see *Uses*).

**Are instances related to each other?** Yes — the 1,016 derived examples
(1,270 − 254) are matched pairs anchored to their 254 source seeds. A
derived failure shares the seed's corpus and prompt world, differing only
by the single injected operator. This matched-pair structure is the basis
of the contrast-set methodology.

**Recommended data splits.** None. The dataset is an evaluation benchmark
used in full; there is no train/test split because nothing is trained on
it.

**Errors, noise, redundancies.** Bullet-glyph, hyphen-break, and
line-split artifacts from PDF extraction are preserved verbatim in some
spans (e.g. `▇` separators, `lifethreatening`). The faithfulness audit
treats these as benign reconstruction artifacts, not errors (see
*Preprocessing/labeling* and *Limitations*). The seeds are stylistically
homogeneous because they were authored against one fixed guideline by AI
sub-agents — itself a noise/representativeness concern, disclosed under
*Limitations*.

**Self-contained vs. external resources.** The shipped TOML is
self-contained. The 50 source PDFs are public-domain DailyMed labels
fetched via the public DailyMed v2 API; the fetch manifest records SHA-256
hashes so the corpus is reproducible/verifiable. DailyMed labels are
public-domain U.S. government / FDA-mandated content; there is no
confidential or personally identifying information.

---

## Collection Process

**How was the data acquired?**

- **Source corpus.** `evidence-eval fetch dailymed --limit 50` walks the
  public DailyMed v2 API and downloads SPL label PDFs idempotently, writing
  a `manifest.toml` with stable set IDs, **SHA-256** per file, and fetch
  timestamps. Reruns skip files already on disk that match the manifest, so
  the corpus is reproducible.
- **Span extraction.** `evidence-eval author` extracts text from each PDF
  **line-level** via PDFium (one readable run per span), emitting candidate
  `corpus_spans` scaffolding. Spans are line-level, not per-glyph; a single
  label sentence is therefore split across consecutive line entries.
- **Seed authoring — a deterministic pipeline under a fixed published
  guideline (required reading for a reviewer).** The 254 `valid` seed
  examples were produced by an automated authoring pipeline: they are
  **AI-authored, not human-written.** Large-language-model agents ran as
  **parallel sub-agents**, each driven by the `evidence-eval author` span
  scaffold over one PDF, under a single fixed authoring guideline
  (`docs/EVAL.md`) that is published in-repo and pins the procedure
  exactly. For each example the agent: selected and concatenated
  the consecutive line-level spans forming one complete sentence into a
  single `corpus_span` (single-space join, CRLF trimmed, verbatim words,
  no paraphrase or invention); wrote a `response_sentence` that is a
  faithful restatement entailed by that span alone; and wrote two typed
  `support_mutations` with varied mechanisms (number flip / negation /
  entity swap / polarity reversal), one `contradicted` and one
  `unsupported`. Because the guideline is fixed and published and the
  span scaffold is deterministic, the authoring procedure is
  reproducible and inspectable end-to-end — and every cited span was
  subsequently checked verbatim against the source PDFs by the automated
  faithfulness audit (see *Preprocessing/labeling*), which found zero
  fabricated or unverifiable spans. The seeds were **not independently
  hand-curated**; semantic faithfulness was **spot-audited, not
  exhaustively human-verified**, so the seeds are stylistically and
  structurally **more homogeneous than independent hand curation**. A
  reader must weigh the benchmark accordingly; this is restated, equally
  prominently, under *Limitations*, where the wider human pass is named
  as the explicit next step.

**Over what timeframe was the data collected?** The DailyMed corpus is a
point-in-time fetch pinned by the SHA-256 manifest; the v1→v2 re-authoring
history is recorded in *Preprocessing/labeling*. Exact calendar collection
dates are not stated in the source documents.

**Were the individuals notified / did they consent?** Not applicable. The
source corpus is public-domain FDA-mandated drug labeling; no human
subjects or personal data are involved.

**Ethical review.** Not applicable — public-domain regulatory text only.

---

## Preprocessing / Cleaning / Labeling

**Was preprocessing done?**

- **Structural validation.** Every authored file passed `evidence-eval
  lint`; whole-directory lint reports **0 diagnostics**. `lint` verifies
  *structure* (well-formed TOML, schema, ID references), **not** semantic
  entailment — explicitly noted as a limitation below.
- **v1 → v2 fragment-citation fix.** v1 (263 seeds) used a flawed
  guideline ("cite minimally, one span"). On line-wrapped PDF text this
  produced *fragment* citations — e.g. citing
  `"Doxycycline is virtually completely"` while the word *absorbed* sat in
  the next, uncited line. A hand audit of v1's false-refused `valid`
  examples against the source PDFs established this was a benchmark
  *authoring bug*, not an NLI finding: it inflated the false-refusal rate
  by ~15 points (≈49% → ≈34% real). The fix corrected the guideline
  (`docs/EVAL.md`) to require **one complete, self-contained sentence** per
  cited span and **re-authored all seeds**. v1 is preserved out-of-repo at
  `~/evidence-bench-v1/` (not committed). v2 is the shipped dataset (254
  seeds).
- **Labeling / failure injection (named operators).** The 1,270 labeled
  examples are materialized by `evidence-eval inject`
  (`crates/evidence-eval/src/inject.rs`). Each `valid` seed yields:
  - an **existence** variant — citation rewritten to a span ID absent from
    the corpus (→ `fabricated_span`);
  - an **in-context** variant — a duplicate-text span added at a fresh ID
    *outside* the prompt, with the citation repointed to it (→
    `out_of_context`);
  - one **support** variant per `support_mutations` entry — the
    author-supplied sentence rewrite, labeled `unsupported` or
    `contradicted`.
  Operators are named and construction is deliberately explicit so each
  can be cited individually (contrast sets, Gardner et al. 2020).

**Was the raw data saved?** Yes — the source PDFs and SHA-256 manifest are
the immutable raw layer; v1 seeds are retained out-of-repo for audit.

**Automated faithfulness audit.** `evidence-eval audit-faithfulness`
checks every cited span verbatim against the source corpus. On v2
(`docs/paper/faithfulness-audit.md`): over **254 examples / 255 corpus
spans** — **249 exact, 1 fuzzy, 5 missing (flagged)**. All six flags
(`hamptonsun_reapply_swimming`, `mucinex_max_daily_amount`, `dermfree_use`,
`noxivent_indication`, `cmc_eyedrops_storage`, `amiodarone_reserve_use`)
were manually verified against the source PDFs and found to be **faithful
real label text** — flagged only because the audit tool does not collapse
interior whitespace or merge bullet-glyph / line-split / hyphen-break
extraction artifacts. In particular `cmc_eyedrops_storage` was located
verbatim in its source label (`51a2d74f-…345a.pdf`, page 1, span id 1320),
the earlier grep having missed it solely because the source has three
interior spaces between the two sentences. **No fabricated or
unverifiable spans remain; no span text was changed and no example was
removed.** Structural results (existence/in-context, non-overlap) do not
depend on span text and are unaffected.

---

## Uses

**What is the dataset intended to be used for?**

Measuring **validator-boundary citation correctness**: given a labeled set
of well-formed and deliberately-broken citations, how does each gate, and
the composed validator, behave? The headline measurements supported on
this benchmark:

- **Per-rule isolation (§5.2).** existence F1 = 1.000 [1.00, 1.00] (tp/fp/fn
  254/0/0); in-context F1 = 1.000 [1.00, 1.00] (254/0/0); support F1 =
  0.929 [0.91, 0.94], precision 0.873, recall 0.992 (504/73/4). (95%
  bootstrap CI, B=1000, fixed seed, over the 1,270 examples.)
- **Non-overlap (§5.3).** OutOfContext accepted by existence-only:
  254/254 (100.0%); Unsupported+Contradicted accepted by two-gate:
  508/508 (100.0%) — a *measured blindness* property, not a construction
  artifact.
- **Additive lift.** three-gate F1 = 0.963 [0.96, 0.97] vs. strongest
  non-composed baseline (existence-only) F1 = 0.400 → **+56.3 F1 points**.
- **NLI comparison (§5.1).** valid-retention: v2 distilbert 66%
  (167/254), v2 DeBERTa 71% (181/254); all-class three-gate 1106/1270
  (distilbert) → 1166/1270 (DeBERTa), Δ +60.

**What should the dataset *not* be used for? (explicit non-uses)**

- **Not a causal-faithfulness benchmark.** It measures citation-boundary
  correctness against a known label, not whether a model's reasoning
  causally depends on the cited evidence.
- **Not a measure of real-world hallucination prevalence.** It is a
  **class-balanced, injected** set (254 of each evaluated failure class);
  the proportions are an artifact of the matched-pair construction, not an
  estimate of how often real LLMs hallucinate. Every results document
  carries this scope note; the real-prevalence question (natural-failure /
  real-LLM run on a held-out corpus, no injection) is tracked as open work
  (§5.6 in the paper draft) and is explicitly out of scope here.
- **Not a multi-span / composite-claim benchmark.** Single-span dominance
  (see *Composition*) means clause-level decomposition and
  concatenated-evidence aggregation are unexercised.
- **Not a domain-general benchmark.** Single domain (FDA drug labels)
  only.

**Anything about composition/collection a consumer should know to avoid
unfair use?** The AI-assisted authoring and the non-exhaustive human
verification (below) must be reported alongside any result derived from
this dataset. The class-balanced design must never be presented as a
prevalence estimate.

---

## Distribution

**Will the dataset be distributed?** Yes — it ships in the open-source
`evidence` repository at
`crates/evidence-eval/datasets/fda_label_bench.toml`.

**How?** As an in-repo TOML file. The source corpus is not redistributed
in-repo but is reproducible via `evidence-eval fetch dailymed --limit 50`
against the public DailyMed v2 API; the SHA-256 manifest pins exact files.

**License / IP.** The repository is licensed under the **Apache License
2.0** (`LICENSE`). The underlying source corpus is **public-domain** U.S.
FDA / DailyMed SPL labeling, carrying no third-party copyright
restrictions. No IP, confidentiality, or export controls apply.

**Third-party restrictions / fees.** None. The DailyMed v2 API is public;
no fees or access controls.

---

## Maintenance

**Who maintains the dataset?** The `evidence` project maintainers, via the
repository.

**How can it be updated / who contributes?** Through the repository's
normal change process. The provenance and audit trail
(`fda_label_bench_PROVENANCE.md`, `benchmark-results.md`,
`faithfulness-audit.md`) are versioned alongside the data; the v1→v2 arc
is deliberately retained rather than rewritten so the audit history stays
inspectable.

**Versioning.** The shipped dataset is **v2**; v1 (263 seeds) is preserved
out-of-repo at `~/evidence-bench-v1/`. Any future re-authoring should
likewise be versioned with a documented audit, and the faithfulness audit
re-run.

**Will older versions be supported?** v1 is retained for audit
reproducibility only; v2 supersedes it for all reported results.

**Will the dataset be extended?** The recommended extensions are: a wider
human verification pass over the seeds (semantic entailment, not only span
faithfulness); inter-annotator agreement; multi-span examples; and a
natural-failure / real-LLM run for prevalence (§5.6). These are documented
as open work, not yet done.

---

## Limitations (read this before citing any result)

- **AI-assisted authoring.** The 254 `valid` seeds were authored by large-language-model
  sub-agents under a single fixed guideline, not independently
  hand-curated. This makes the seeds **more stylistically and
  structurally homogeneous** than an independently human-written
  benchmark and is the primary threat a reviewer should weigh. It is
  disclosed in *Motivation*, *Collection process*, and here, by design.
- **Non-exhaustive human verification.** `lint` checks structure, not
  semantic entailment. v2 semantic faithfulness was **spot-audited on 3
  labels** (amiodarone, dapagliflozin, doxycycline), plus the full v1
  false-refusal hand audit, plus the automated `audit-faithfulness` pass
  with manual resolution of all 6 flags (249 exact / 1 fuzzy / 5 flagged —
  all verified faithful real label text). There has been **no exhaustive
  human pass over all 254 seeds** for semantic entailment. A wider human
  verification pass is the conservative path for publication and is
  explicitly recommended.
- **No inter-annotator agreement.** No second annotator or IAA statistic
  exists; this is acknowledged open work.
- **Single domain.** FDA drug labels only — no claim of generality to
  other regulated or non-regulated corpora.
- **Class-balanced, injected — not prevalence.** 254 of each evaluated
  failure class by construction; results characterize validator behaviour
  under matched-pair injected failures, never real-world failure rates.
- **Single-span dominance.** Composite/multi-span claims and clause-level
  decomposition are unrepresented.
- **Single configuration.** One retrieval configuration; the structural
  gates are model-independent, but the support-gate numbers reflect the
  specific NLI checkpoint of the run (default
  `Xenova/distilbert-base-uncased-mnli`; candidate
  `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`). No latency numbers are
  part of this dataset.
- **Benign extraction artifacts retained.** Bullet-glyph, hyphen-break,
  and interior-whitespace artifacts are preserved verbatim; all such
  flagged spans were manually confirmed faithful to source.

**Net.** This is a strong scaffold benchmark whose defensibility rests on
a rigorous, reproducible process and full candor about its scope. The
authoring pipeline is deterministic and runs under a fixed published
guideline; the automated faithfulness audit verified every cited span
against source with zero fabrications; the v1→v2 bug was self-caught,
documented, and corrected; and the structural results
(existence / in-context, non-overlap) are authoring-independent by
construction. The limitations above — AI-assisted authoring, the
not-yet-exhaustive human semantic verification, no IAA, single domain,
class-balanced injection, single-span dominance, single configuration —
are real, scoped, and stated here in full and at equal prominence so
that results derived from this dataset are read with the appropriate
caveats; several are already partially mitigated by the audit trail. We
hold that disclosed, auditable, reproduced authoring is a stronger basis
for trust than an unverifiable claim of manual curation. The explicit
next step before publication-grade claims is an independent human
verification pass over the seeds and an inter-annotator agreement
statistic.
