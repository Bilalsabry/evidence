# §5 Experiments — DRAFT

> **DRAFT — not for submission as-is.** Author must finalize voice and
> re-verify every number against the source tables before any external
> use. Every figure here is produced by a tested, reproducible tool
> (`evidence-eval metrics` / `compare` / `run`) and traces to a
> committed artifact (`rule-metrics.md`, `nli-comparison.md`,
> `benchmark-results.md`, `fda_label_bench_PROVENANCE.md`).
> Subsections **5.5–5.6 are not yet run** and are marked `[OPEN]` — do
> not publish them as written. 5.7 (cost) is measured.

## 5.0 Benchmark construction and audit

The benchmark (`fda_label_bench.toml`, v2) is 254 hand-shaped `valid`
seed examples drawn from 50 public-domain DailyMed FDA drug labels,
expanded to 1,270 labeled examples by the matched-pair injection
operators of §4: two structural injections (existence, in-context) and
two support mutations (contradicted, unsupported) per seed. Seeds were
AI-assisted-authored under a fixed guideline and verified structurally
by `evidence-eval lint` (0 whole-corpus diagnostics); provenance and
limitations are in the datasheet (`fda_label_bench_PROVENANCE.md`).

One methodological point bears on the validity of §5.2–5.4. An initial
benchmark (v1) showed a 49% false-refusal rate on `valid` examples at
the full validator. A hand audit of the refused examples against source
PDFs traced the cause to an authoring artifact, not a property of the
validator: the span extractor emits line-level spans, label sentences
wrap across lines, and the v1 guideline produced *fragment* citations
(a cited span missing the subject or the operative number). Both a weak
and a strong NLI model correctly declined to entail a sentence from a
fragment. We corrected the guideline (the cited span must be one
complete, self-contained sentence), re-authored all seeds (v2), and
repeated the run. The v1→v2 delta is recorded in
`benchmark-results.md`; we keep it on record because catching this by
audit, rather than shipping it, is part of the evidence that the
remaining numbers are real.

## 5.1 Setup

Validator policies form a nested stack: `vanilla` (no validation) ⊂
`existence_only` (rule 1) ⊂ `two_gate` (rules 1+2) ⊂ `three_gate`
(rules 1+2+3). Rules 1 and 2 are deterministic structural checks and
are model-independent. Rule 3 (support) is a natural-language inference
check; we evaluate two MNLI checkpoints: a distilled BERT baseline
(`distilbert-base-uncased-mnli`) and a stronger DeBERTa-v3 model
(`DeBERTa-v3-base-mnli-fever-anli`, ONNX). Retrieval configuration:
`[OPEN]`. Latency: measured, §5.7.

Under the stronger model the validator agrees with the labeled outcome
matrix on 1,166 / 1,270 three-gate decisions; the residual is
concentrated in support-gate precision (model effect, below), not in
the structural rules, which are exact.

**Model effect.** Replacing distilbert with DeBERTa-v3 moves three-gate
agreement from 1,106 to 1,166 / 1,270 (+60). Valid-retention rises from
167 to 181 / 254, contradicted catch from 212 to 244 / 254, and
unsupported from 219 to 233 / 254. The structural classes are unchanged
(Δ = 0), confirming rules 1 and 2 do not depend on the NLI model (full
table: `nli-comparison.md`). The stronger model helps but does not
eliminate support-gate conservatism — ~29% of true,
complete-sentence-cited claims are still refused — which motivates
treating the support gate as a separately measured, conservative
component rather than folding it into a single citation score.

## 5.2 Per-rule isolation

Each rule is scored as its marginal gate over the 1,270-example set:
the examples it refuses among those the previous policy accepted, with
the class it owns as the positive label.

| rule | owned class | precision | recall | F1 |
|---|---|---|---|---|
| existence | fabricated_span | 1.000 | 1.000 | **1.000** |
| in-context | out_of_context | 1.000 | 1.000 | **1.000** |
| support | unsupported + contradicted | 0.873 | 0.992 | 0.929 |

On the v2 injected FDA-label benchmark (not real-world failure
prevalence), existence and in-context are exact and model-independent:
every fabricated span is caught at the existence gate and every
out-of-context citation at the in-context gate, with no false positives
and no misses. The support rule recalls 0.992 of genuine support
failures (it does not *miss* bad citations); its F1 is bounded by
precision 0.873, where the 73 false positives are valid claims the NLI
checkpoint conservatively refuses. That precision figure is the §5.1
finding, not a flaw in the decomposition. Source: `rule-metrics.md`.

## 5.3 Non-overlap (the kernel)

The central claim is that the three rules are not redundant
re-measurements of one signal but catch structurally disjoint classes
of error. We measure this directly: whether each failure class is
visible to the rules that precede its owning gate.

On the v2 injected FDA-label benchmark (not real-world failure
prevalence):

- Out-of-context citations accepted by existence-only: **254 / 254
  (100%)** — the existence rule is entirely blind to in-context errors.
- Support failures (unsupported + contradicted) accepted by two-gate:
  **508 / 508 (100%)** — rules 1 and 2 are entirely blind to support
  errors.

The error classes are fully disjoint (100% by the operational
blindness measure): no failure is caught by a rule other than the one
that owns it. This is a *measured blindness property*, not an artifact
of the nested-policy construction — out-of-context citations are
invisible to existence-only because the cited span genuinely exists,
and support failures are invisible to two-gate because the span exists
and is in context; each gate is blind to the others' classes by what it
checks, not by how the policies are layered. The decomposition is
therefore not a partition of convenience: the rules address independent
failure modes, and omitting any one leaves its class entirely
undetected.

## 5.4 Composition and ablation

Treating each policy as a binary should-refuse classifier over the
injected set:

| policy | precision | recall | F1 |
|---|---|---|---|
| vanilla (no validation) | 1.000 | 0.000 | 0.000 |
| existence_only | 1.000 | 0.250 | 0.400 |
| two_gate (1+2) | 1.000 | 0.500 | 0.667 |
| three_gate (1+2+3) | 0.933 | 0.996 | **0.963** |

The ladder is monotone and each gate contributes a large increment:
recall roughly doubles as each rule is added (0.25 → 0.50 → 0.996),
while precision stays 1.000 through two-gate and dips only to 0.933 at
three-gate (the §5.1 false-refusals). On the v2 injected FDA-label
benchmark (not real-world failure prevalence) the composed validator
improves should-refuse F1 by **+56.3 points** over the best
non-composed baseline (existence-only). Because the policy stack is
nested, only `vanilla` and `existence_only` are genuinely single-rule;
in-context-alone and support-alone are not isolable, so this baseline
is the strongest *available* single rule — a conservative choice that
makes the reported lift a lower bound. Source: `rule-metrics.md`.

## 5.5 Comparison to ALCE `[OPEN — not run]`

Plan: translate a ~50-example ALCE subset into our framework and show
the composed validator scores comparably or better on ALCE's own
metrics, establishing the decomposition is not an artifact of our
benchmark. Not yet executed; do not draft numbers.

## 5.6 Natural-failure run `[OPEN — not run]`

Plan: real LLM hallucinations (no injection) on the corpus, measuring
per-rule catch rate and human-eval agreement (CLAIMS.md claim 4). This
requires a held-out corpus, a chosen generation model, and human
grading — a methodological decision and a human-labeling effort, not an
automatable step. Not yet executed.

## 5.7 Cost analysis

Harness-level wall-clock decomposition over the 1,270-example set
(`cost-latency.md`, reproducible via `evidence-eval latency`):

| stage | mean / call | share |
|---|---|---|
| structural (existence + in-context + harness) | 140 µs | 2.5% |
| support gate (NLI forward pass) | 5.6 ms | 97.5% |

The structural rules are effectively free; cost is dominated by the
support NLI forward pass (~40× the structural path). This is the
deployment argument for the decomposition: the two exact gates can run
unconditionally on every citation, with the expensive semantic gate
applied only where the cheap ones pass. Measurement is harness-level
(mock-run baseline vs. real-NLI delta), order-of-magnitude not
microbenchmark — method note in `cost-latency.md`. Existence vs.
in-context are not separated (both O(µs), dwarfed by the NLI pass).

## Threats to validity (carry into §6)

- Seeds are AI-assisted-authored from a fixed guideline; more
  structurally homogeneous than independent hand curation. v2
  entailment was spot-audited (3 labels) plus the full v1 false-refusal
  audit, not an exhaustive human pass over all 254. (Datasheet.)
- Injection is by construction; §5.6 is the intended defense and is not
  yet run.
- One NLI checkpoint pair, one retrieval configuration, no
  inter-annotator agreement, no latency numbers.
- §5.2/§5.4 support-row figures depend on the NLI model; the structural
  results (§5.2 existence/in-context, all of §5.3) do not.
