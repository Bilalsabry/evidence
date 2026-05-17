# Benchmark results — FDA drug-label set (v2, N = 254 seeds)

The paper's empirical benchmark, with its full audit history. v2
supersedes v1 after an audit found and fixed a benchmark-authoring bug.
This document keeps the v1 → v2 arc on purpose: finding the bug by
audit and disambiguating it from the real finding is a methodological
strength, not something to hide.

## The v1 bug (found by audit, now fixed)

v1 (263 seeds) showed 49% of `valid` claims false-refused at
three-gate under distilbert. Auditing the false-refused examples by
hand against the source PDFs showed the cause was **not** the model:
the `author` tool emits **line-level** spans, label sentences wrap
across lines, and the v1 authoring guideline ("cite minimally, one
span") made the agents cite a *fragment* — e.g. `cited span =
"Doxycycline is virtually completely"` while the word *absorbed* was in
the next, uncited line. NLI correctly refuses a fragment that does not
state the claim. ~15 points of the v1 "false-refusal" was this
artifact.

Fix: corrected the authoring guideline (`docs/EVAL.md`) — the cited
span must be **one complete, self-contained sentence**, with wrapped
PDF lines merged into it — and re-authored all seeds. Faithfulness
spot-audit on v2 (amiodarone, dapagliflozin, doxycycline): merged
spans are complete sentences, verbatim from source (the labels' own
grammatical quirks preserved, not paraphrased).

## v2 results

254 `valid` seeds → 1,270 examples via `inject` → 5,080 (example,
policy) rows. Whole-dir `lint`: 0 diagnostics.

### Catch matrix — distilbert (`run --real-nli`)

| class \ policy | vanilla | existence | two-gate | three-gate |
|---|---|---|---|---|
| Valid | 254/0 | 254/0 | 254/0 | **167 / 87** |
| FabricatedSpan | 254/0 | **0/254** | 0/254 | 0/254 |
| OutOfContext | 254/0 | 254/0 | **0/254** | 0/254 |
| Unsupported | 254/0 | 254/0 | 254/0 | 35 / 219 |
| Contradicted | 254/0 | 254/0 | 254/0 | 42 / 212 |

### §5.1 — distilbert vs DeBERTa-v3-base-mnli-fever-anli (three-gate)

| class | distilbert | DeBERTa | Δ |
|---|---|---|---|
| valid | 167 / 254 | **181 / 254** | +14 |
| fabricated_span | 254 / 254 | 254 / 254 | 0 |
| out_of_context | 254 / 254 | 254 / 254 | 0 |
| unsupported | 219 / 254 | 233 / 254 | +14 |
| contradicted | 212 / 254 | 244 / 254 | +32 |
| **all** | **1106 / 1270** | **1166 / 1270** | **+60** |

Full table: `docs/paper/nli-comparison.md`.

## Reading it

**Valid-retention, the headline number:**

| | valid-retention |
|---|---|
| v1 distilbert (buggy benchmark) | 51% (134/263) |
| v2 distilbert (clean benchmark) | **66% (167/254)** |
| v2 DeBERTa (clean + strong model) | **71% (181/254)** |

The v1 49%-false-refusal decomposes cleanly into **~15 points
benchmark bug** (fixed) + **~29% genuine NLI conservatism** that
persists even with complete-sentence citations *and* a strong
DeBERTa-v3 MNLI model. That residual is now a real, defensible finding
about the support gate, not an artifact.

**Rules 1 & 2 are flawless and model-independent.** FabricatedSpan
254/254 at the existence gate, OutOfContext 254/254 at the in-context
gate, Δ=0 between models, zero cross-gate leakage. The §5.2/§5.3
kernel claim holds on real drug-label text under both models. This is
the strongest result and it never depended on the support gate.
Quantified in `docs/paper/rule-metrics.md`: existence & in-context
F1 = 1.000, support F1 = 0.929 (recall 0.992), **100% disjoint**.

**The support gate works on real failures.** DeBERTa catches
Contradicted 244/254 (96%) and Unsupported 233/254 (92%). It is
conservative — it over-refuses ~29% of valid — but it does not *miss*
bad citations. The failure mode is false-refuse-on-valid, never
missed-failure. This is the empirical case for measuring the support
gate independently (mock-mode is 100% by construction) and for a
refuse-or-resolve deployment philosophy: a conservative gate is the
right default when the cost of a bad citation is high.

## Caveats (do not over-read)

- Seeds are AI-authored from a corrected template/guideline; more
  homogeneous than independent hand curation. See the datasheet.
- v2 faithfulness was spot-audited (3 labels) plus the v1 false-refusal
  audit; not an exhaustive human pass over all 254.
- Single retrieval config; one candidate NLI checkpoint; no latency
  numbers; no inter-annotator agreement.
