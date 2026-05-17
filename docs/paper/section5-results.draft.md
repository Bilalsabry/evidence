# §5 Experiments — DRAFT

> **Status: machine-drafted from the final in-repo numbers, for the
> author to revise.** Every figure here is produced by a tested,
> reproducible tool (`evidence-eval metrics` / `compare` / `run`) and
> traces to a committed artifact (`rule-metrics.md`,
> `nli-comparison.md`, `benchmark-results.md`,
> `fda_label_bench_PROVENANCE.md`). Subsections **5.5–5.7 are not yet
> run** and are marked `[OPEN]` — do not publish them as written.
> Prose is a starting point, not final; tighten, re-voice, and verify
> against the source tables before submission.

## 5.0 Benchmark construction and audit

The benchmark (`fda_label_bench.toml`, v2) is 254 hand-shaped `valid`
seed examples drawn from 50 public-domain DailyMed FDA drug labels,
expanded to 1,270 labeled examples by the matched-pair injection
operators of §4: two structural injections (existence, in-context) and
two support mutations (contradicted, unsupported) per seed. Seeds were
AI-assisted-authored under a fixed guideline and verified structurally
by `evidence-eval lint` (whole-corpus: 0 diagnostics); provenance and
limitations are documented in the datasheet
(`fda_label_bench_PROVENANCE.md`).

We report one methodological point because it bears on the validity of
§5.2–5.4. An initial benchmark (v1) showed a 49% false-refusal rate on
`valid` examples at the full validator. A hand audit of the refused
examples against source PDFs found the cause was an authoring artifact,
not a property of the validator: the span extractor emits line-level
spans, label sentences wrap across lines, and the v1 guideline produced
*fragment* citations (a cited span missing the subject or the operative
number). Both a weak and a strong NLI model correctly declined to
entail a sentence from a fragment. The guideline was corrected (the
cited span must be one complete, self-contained sentence), all seeds
re-authored (v2), and the run repeated. The v1→v2 delta is reported in
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
(`DeBERTa-v3-base-mnli-fever-anli`, ONNX). Retrieval configuration and
latency: `[OPEN — 5.7]`.

Under the stronger model the validator agrees with the labeled outcome
matrix on 1,166 / 1,270 three-gate decisions; the residual is
concentrated in support-gate precision (§5.1 model effect, below), not
in the structural rules, which are exact.

**Model effect (§5.1).** Replacing distilbert with DeBERTa-v3 moves
three-gate agreement from 1,106 to 1,166 / 1,270 (+60). Valid-retention
rises from 167 to 181 / 254; contradicted catch from 212 to 244 / 254;
unsupported from 219 to 233 / 254. The structural classes are
unchanged (Δ = 0), confirming rule 1 and rule 2 do not depend on the
NLI model. Full table: `nli-comparison.md`. The stronger model helps
but does not eliminate support-gate conservatism — ~29% of true,
complete-sentence-cited claims are still refused — which is the
motivation for treating the support gate as a separately measured,
conservative component rather than folding it into a single citation
score.

## 5.2 Per-rule isolation

Each rule is scored as its marginal gate over the 1,270-example set:
the examples it refuses among those the previous policy accepted, with
the class it owns as the positive label.

| rule | owned class | precision | recall | F1 |
|---|---|---|---|---|
| existence | fabricated_span | 1.000 | 1.000 | **1.000** |
| in-context | out_of_context | 1.000 | 1.000 | **1.000** |
| support | unsupported + contradicted | 0.873 | 0.992 | 0.929 |

Existence and in-context are exact: every fabricated span is caught at
the existence gate and every out-of-context citation at the in-context
gate, with no false positives and no misses. These two results are
model-independent. The support rule recalls 0.992 of genuine support
failures (it does not *miss* bad citations); its F1 is bounded by
precision 0.873, where the 73 false positives are valid claims the NLI
checkpoint conservatively refuses. That precision figure is the §5.1
finding, not a flaw in the decomposition. Source: `rule-metrics.md`.

## 5.3 Non-overlap (the kernel)

The central claim is that the three rules are not redundant
re-measurements of one signal but catch structurally disjoint classes
of error. We measure this directly: whether each failure class is
visible to the rules that precede its owning gate.

- Out-of-context citations accepted by existence-only: **254 / 254
  (100%)** — the existence rule is entirely blind to in-context errors.
- Support failures (unsupported + contradicted) accepted by two-gate:
  **508 / 508 (100%)** — rules 1 and 2 are entirely blind to support
  errors.

The error classes are perfectly disjoint: no failure is caught by a
rule other than the one that owns it. The decomposition is therefore
not a partition of convenience; the rules address independent failure
modes, and omitting any one leaves its class entirely undetected.

## 5.4 Composition and ablation

Treating each policy as a binary should-refuse classifier over the
injected set:

| policy | precision | recall | F1 |
|---|---|---|---|
| vanilla (no validation) | 1.000 | 0.000 | 0.000 |
| existence_only | 1.000 | 0.250 | 0.400 |
| two_gate (1+2) | 1.000 | 0.500 | 0.667 |
| three_gate (1+2+3) | 0.933 | 0.996 | **0.963** |

The ladder is monotone and each gate contributes a large increment;
recall roughly doubles as each rule is added (0.25 → 0.50 → 0.996)
while precision stays 1.000 through two-gate and dips only to 0.933 at
three-gate (the §5.1 false-refusals). The composed validator improves
should-refuse F1 by **+56.3 points** over the strongest non-composed
baseline (existence-only). Because the policy stack is nested, only
`vanilla` and `existence_only` are genuinely single-rule; in-context-
alone and support-alone are not isolable, so this baseline is the
strongest *available* single rule — a conservative choice that makes
the reported lift a lower bound. Source: `rule-metrics.md`.

## 5.5 Comparison to ALCE `[OPEN — not run]`

Plan: translate a ~50-example ALCE subset into our framework and show
the composed validator scores comparably or better on ALCE's metrics,
establishing the decomposition is not a benchmark artifact. Not yet
executed; do not draft numbers.

## 5.6 Natural-failure run `[OPEN — not run]`

Plan: real LLM hallucinations (no injection) on the corpus, per-rule
catch rate, human-eval agreement (CLAIMS.md claim 4). This requires a
held-out corpus, a chosen generation model, and human grading — a
methodological decision and a human-labeling effort, not an automatable
step. Not yet executed.

## 5.7 Cost analysis `[OPEN — not measured]`

Plan: per-rule latency, cumulative validator overhead vs. retrieval.
Structural rules are O(1) lookups; the cost is dominated by the NLI
forward pass in rule 3. Not yet measured.

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
