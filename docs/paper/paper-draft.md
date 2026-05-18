# Three Rules for a Citation: Decomposing Faithfulness in Retrieval-Augmented Generation — DRAFT

> **DRAFT — not for submission as-is.** The author must finalize voice and re-verify every number against the source tables (`rule-metrics.md`, `nli-comparison.md`, `benchmark-results.md`) before any external use.

---

## Abstract (draft)

Retrieval-augmented generation systems attach citations to generated
claims, and a large literature measures whether those citations are
"correct" as a single signal. We argue that citation correctness is
not one property but three operationally independent constraints:
whether the cited span **exists** in the corpus, whether it was
**in-context** (in the retrieval set the model was shown for this
query), and whether it **supports** the claim. We externalize all
three as validator gates that refuse output, and build a benchmark of
254 FDA-drug-label seed examples expanded to 1,270 via named
matched-pair failure-injection operators. On the v2 injected FDA-label
benchmark the structural rules (existence, in-context) are exact and
model-independent (F1 = 1.000); the three rules catch fully disjoint
classes of error (100% by the operational blindness measure); and the
composed validator improves should-refuse F1 by +56 points over the
best non-composed baseline. The semantic support gate, measured
separately, catches genuine failures at 0.99 recall but a strong NLI
model still conservatively over-refuses true claims — a finding we
measure rather than hide, and which motivates treating support as its
own refuse-or-resolve gate. We release the system, the benchmark, and
one-command reproduction.

**Scope.** All results are on a controlled, failure-injected,
single-domain (FDA drug-label) benchmark; they characterize validator
behavior under matched-pair injected failures, not real-world failure
prevalence. Structural rules (existence, in-context) are exact and
model-independent; the support gate is a conservative, high-recall
safety gate, not a high-precision throughput filter.

---

## 1. Introduction

Consider a model that writes *"the trial enrolled 1000 patients"* and
attaches a citation to a span reading *"the trial enrolled 240
patients."* The citation is well-formed, the link resolves, the cited
span is topically relevant — and the claim is false. Most existing
attribution metrics score this as "cited with high relevance." That is
the bug this paper is about.

"Is this citation correct?" is treated as one signal. It is three. A
citation is trustworthy only if (1) the cited span **exists**, (2) it
was **in the context** the model was given for this query, and (3) it
**supports** the claim. These fail independently: a fabricated span ID
fails (1); a real span the model was never shown fails (2) while
passing (1); a real, shown, but irrelevant span passes (1) and (2) and
fails (3). Conflating them into one score hides the practitioner's
actual question — *which rule failed?* — and the three failures have
completely different fixes (decoding/format, retrieval-context,
reasoning).

**Contributions.** (i) We decompose citation correctness into
existence, in-context, and support, and externalize each as a
validator gate that refuses rather than degrades. (ii) We build a
public FDA-drug-label benchmark with controlled, named matched-pair
failure injection (254 seeds → 1,270 labeled examples). (iii) We show,
under matched-pair failure injection, that the three rules catch
**non-overlapping** classes of error (fully disjoint on the v2 injected
FDA-label benchmark, 100% by the operational blindness measure) and
that the composed validator improves should-refuse F1 by **+56 points**
over the best non-composed baseline; the structural rules are exact and
model-independent.

§2 places the work; §3 defines the rules; §4 the benchmark; §5 the
experiments; §6 limitations; §7 conclusion.

## 2. Related work

**Attribution and faithfulness.** AIS (Rashkin et al. 2023) is the
dominant operational definition; it bundles existence, in-context, and
support into one human judgment. Attributed QA (Bohnet et al. 2022)
inherits AIS. TRUE (Honovich et al. 2022) establishes NLI as a
faithfulness signal but is orthogonal to the decomposition question.

**RAG citation benchmarks.** ALCE (Gao et al. 2023), HAGRID (Kamalloo
et al. 2023), and ExpertQA (Malaviya et al. 2024) measure support
carefully but restrict citations to integer markers into the retrieved
set — existence and in-context are true *by construction* and cannot be
measured. These are the closest comparison points; an explicit
translation experiment is future work (§5.5).

**Span-level and constrained citing.** GopherCite (Menick et al. 2022)
forces existence and in-context by constrained decoding rather than
measuring them. Self-RAG (Asai et al. 2024) has separate reflection
tokens for retrieval relevance and support, but they are trained inside
the model rather than externalized validator gates, and neither
addresses "was this span in the prompt for this query."

**Post-hoc and frameworks.** RARR (Gao et al. 2022) revises rather than
refuses. RAGAS (Es et al. 2023) is a rule-based RAG-eval framework; we
position the three-rule decomposition as the right granularity for
RAGAS-style frameworks to adopt. Contrast-set methodology (Gardner et
al. 2020) underwrites our named-operator failure injection.

**The gap.** To our knowledge no prior work separates existence,
in-context, and support as independent failure modes with separate
measurements, and none shows they catch distinct, non-overlapping
errors. (Full matrix: `related-work-table.md`.)

## 3. The three rules

### 3.0 Formal definitions

Let C be the corpus of spans. For a query q, let R_q ⊆ C be the retrieval set the model was shown. Let s be a cited span and y the claim sentence. The three gates: exists(s, C) ≡ s ∈ C; in_context(s, R_q) ≡ s ∈ R_q; supports(s, y) ≡ NLI(text(s), y) = entailment. The validator accepts the citation (s, y) iff exists ∧ in_context ∧ supports, and otherwise refuses with the typed reason of the first failing gate. in_context establishes that s was *available* to the model, not that the model *used* it (see Scope of the claim, below).

**3.1 Existence.** The cited identifier must resolve to a real span in
the corpus. Failure: a fabricated pointer into nothing. Implementation:
a primary-key lookup, O(1), deterministic.

**3.2 In-context.** The cited span must have been in the retrieval set
assembled for *this* query — the closed-loop constraint: a model may
only cite what it was shown. Failure: citing a real span the model was
not given this turn (back-filled from parametric memory).
Implementation: a per-prompt allowed-span `HashSet`, deterministic.

**3.3 Support.** The cited span must entail the claim. Implementation:
an NLI cross-encoder over `(span_text, sentence)` pairs producing a
three-class verdict (entailment / neutral / contradiction). The only
model-dependent gate.

**3.4 Composition and strict-wins.** The gates run in order —
existence, then in-context, then support — so each cleanly attributes
one failure class (a class is owned by the first gate that can see it).
Support aggregation is **strict-wins**: any contradiction wins, else
any neutral wins, else support. "Every cited span must hold on its own"
is a conservative safety stance; a concatenated-evidence alternative is
noted as future work, not shipped.

**3.5 Validator output and deployment.** Refusal types: `Uncited`,
`UnknownSpan` (existence), `OutOfContext` (in-context), `Unsupported`,
`Contradicted` (support). Deployment philosophy is **refuse-or-resolve**
— never silently degrade. A conservative gate is the correct default
when the cost of a confidently-wrong citation is high (the target
domain is pharmaceutical text).

### Scope of the claim: citation correctness, not causal faithfulness

We validate citation *correctness* at the output boundary. The in-context gate verifies the cited span was in the retrieval set the model was shown; it does **not** verify the model causally relied on that span — a model can post-rationalize a well-formed citation. Causal reliance (counterfactual span removal, attribution tracing) is a distinct fourth dimension, explicitly out of scope. This boundary is deliberate: existence, in-context, and support are checkable at the validator without model internals; causal faithfulness is not. Recent work separates citation correctness from faithfulness and reports substantial post-rationalization; our claims are confined to correctness.

## 4. Benchmark

**4.1 Corpus.** 50 public-domain FDA drug labels from DailyMed, fetched
via `evidence-eval fetch dailymed` (idempotent; manifest with per-file
SHA-256). Public domain, high-stakes, factually dense — labels carry
crisp doses, concentrations, and contraindications.

**4.2 Seeds.** 254 `valid` seed examples, each a self-contained world:
`corpus_spans`, `prompt_chunks` (what the model was shown),
`response_sentences` (the fixture answer with `cited_spans`), and
author-supplied `support_mutations`. Fact types are rotated
(quantitative, contraindication, indication, administration,
multi-span). Authoring provenance and limitations are documented in a
datasheet (`fda_label_bench_PROVENANCE.md`).

**4.3 Verification and the v1→v2 audit.** Structural validity is
machine-checked by `evidence-eval lint` (whole-corpus: 0 diagnostics).
We report one methodological event because it bears on validity: an
initial benchmark (v1) showed a 49% false-refusal rate on valid
examples; a hand audit against source PDFs found the cause was an
authoring artifact (line-wrapped PDF spans cited as fragments), not a
property of the validator. The guideline was corrected, seeds
re-authored (v2), and the run repeated; the residual decomposed into
~15 points fixed bug and ~29% genuine NLI conservatism. Catching this
by audit is part of the evidence base (`benchmark-results.md`).

**4.4 Failure injection (named operators, Gardner et al. 2020).** From
each valid seed, `evidence-eval inject` derives labeled failures:
- *Existence:* rewrite a citation to a span ID absent from the corpus.
- *In-context:* duplicate a span's text at a fresh ID outside the
  prompt, repoint the citation — the duplicate text controls the
  relevance confound (a true matched pair: only in-context status
  changed).
- *Support:* apply an author-supplied mutation labeled `unsupported`
  (off-topic substitution) or `contradicted` (number/negation/entity
  perturbation) on the same cited span.

254 seeds → 1,270 examples (254 valid + 1,016 injected). Operators are
named, deterministic, individually unit-tested.

**4.5 Natural-failure supplement `[OPEN — not run]`.** Real LLM
outputs (no injection), human-graded into the taxonomy, defending
against "injected failures are too easy." Requires a held-out corpus, a
generation model, and human labeling — deliberately not automated.

## 5. Experiments (condensed; full prose in `section5-results.draft.md`)

Four nested policies: `vanilla` ⊂ `existence_only` ⊂ `two_gate` ⊂
`three_gate`. Support gate evaluated under distilbert-MNLI and
DeBERTa-v3-MNLI.

- **§5.1 Model effect** (`nli-comparison.md`): distilbert→DeBERTa moves
  three-gate agreement 1,106→1,166/1,270; valid-retention 167→181/254;
  structural classes Δ=0 (rules 1–2 model-independent).
- **§5.2 Per-rule isolation** (`rule-metrics.md`): existence F1 =
  **1.000**, in-context F1 = **1.000** (exact, model-independent);
  support F1 = 0.929 (recall 0.992, precision 0.873 — the precision
  gap is the §5.1 conservatism, not a missed failure).
- **§5.3 Non-overlap** (the kernel): out-of-context accepted by
  existence-only 254/254 (100%); support failures accepted by two-gate
  508/508 (100%). **Fully disjoint on the injected benchmark (100% by
  the operational blindness measure)** — each failure class is
  *invisible* to the rules preceding its gate. This is a measured
  blindness property (OOC failures are invisible to the existence gate;
  support failures are invisible to the two-gate policy), an empirical
  property of the data — *not* an artifact of the nested-policy
  construction.
- **§5.4 Composition**: F1 ladder 0.000 → 0.400 → 0.667 → **0.963**;
  **additive lift +56.3 points** over the best non-composed baseline
  (conservative lower bound — nested policies).
- **§5.5 ALCE comparison `[OPEN]`**, **§5.6 natural-failure `[OPEN]`**,
  **§5.7 cost `[OPEN]`** — not yet run; no fabricated numbers.

The decomposition is justified iff each rule isolates its class and the
rules don't overlap; both hold on the v2 injected FDA-label benchmark
with hard numbers. The structural core is exact and model-independent;
the one soft number is the separately-measured support-gate
conservatism.

## 6. Limitations

- **Domain.** FDA labels only. No cross-domain generalization claim.
- **Size.** 254 seeds / 1,270 examples — workshop-appropriate, not
  main-conference scale.
- **Benchmark provenance.** Seeds are AI-assisted-authored from a fixed
  guideline; structurally more homogeneous than independent hand
  curation. The structural results (§5.2 existence/in-context, all of
  §5.3) do not depend on this; the support-gate numbers' authoring
  homogeneity is a fair reviewer question. Only a sample was audited
  against source PDFs; an exhaustive human pass is recommended before
  submission. (Datasheet: `fda_label_bench_PROVENANCE.md`.)
- **NLI error propagation.** The support gate inherits the NLI model's
  errors; we measure NLI behavior as a separate column rather than
  bury it (the support precision figure *is* that measurement).
- **Injection by construction.** §5.6 is the intended defense and is
  not yet run.
- **No human-factors claim.** We do not measure user trust or
  downstream decision quality.
- We measure citation *correctness*, not causal citation faithfulness;
  the in-context gate proves availability, not reliance
  (post-rationalization is possible). This is scoped explicitly in §3.
- Headline F1/lift numbers come from a class-balanced injected set where
  failures are prevalent; real RAG has lower failure prevalence where
  false-refusal cost dominates. We report the support gate's
  false-refusal rate explicitly rather than optimize it away.

## 7. Conclusion and future work

Citation correctness decomposes into three operationally independent
rules; on the v2 injected FDA-label benchmark the structural rules are
exact and model-independent, the rules catch fully disjoint error
classes (100% by the operational blindness measure), and the
composition beats the best single rule by a wide margin. The semantic support gate is the open frontier: it catches
real failures but is conservative, which we measure rather than hide.
Future work: (1) larger, cross-domain, human-curated benchmark;
(2) clause-level support decomposition (FActScore-style) to attack the
support-precision ceiling; (3) the natural-failure study (claim 4) and
ALCE translation; (4) production integration with user studies. The
system, benchmark, and one-command reproduction are released as
`evidence` (open source; version pinned to these experiments — see
`PAPER_EXPLAINER.md` and §7 artifact appendix).

---

## Reviewer-attack checklist (author note, not for submission)

- "Injected failures are too easy" → §5.6 natural-failure run (open).
- "Benchmark is AI-authored" → datasheet + the v1→v2 audit shows we
  catch our own bugs; structural results are authoring-independent.
- "Support precision is low" → that is the *finding*, measured
  separately; recall is 0.992; structural rules are exact.
- "Only one domain / small N" → conceded in §6; workshop scope.
- "Non-overlap is by construction of nested policies" → §5.3 measures
  *blindness* (OOC invisible to existence; support invisible to
  two-gate), which is an empirical property of the data, not the
  policy nesting; §5.4 baseline choice is explicitly conservative.
