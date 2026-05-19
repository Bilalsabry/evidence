# Rule isolation & non-overlap (§5.2 / §5.3)

Marginal-gate decomposition over 1270 examples. Structural rules are model-independent; the support row reflects the NLI checkpoint of this run.

## §5.2 — per-rule isolation
F1 columns carry a 95% bootstrap CI (B=1000, resample-with-replacement over the 1270 examples, fixed seed).

| rule | owned class | precision | recall | F1 (95% CI) | tp/fp/fn |
|---|---|---|---|---|---|
| existence | fabricated_span | 1.000 | 1.000 | 1.000 [1.00, 1.00] | 254/0/0 |
| in-context | out_of_context | 1.000 | 1.000 | 1.000 [1.00, 1.00] | 254/0/0 |
| support | unsupported+contradicted | 0.873 | 0.992 | 0.929 [0.91, 0.94] | 504/73/4 |

## §5.3 — non-overlap (disjoint error classes)

- OutOfContext accepted by existence-only: **254/254 (100.0%)** — the existence rule is blind to in-context errors.
- Unsupported+Contradicted accepted by two-gate: **508/508 (100.0%)** — rules 1 & 2 are blind to support errors.

Each failure class is caught only at the gate that owns it; the preceding rules do not see it. Higher percentages = cleaner disjointness.

## Claim 3 — additive lift (ablation)

Each policy as a binary should-refuse classifier over all 1270 examples (positive = injected failure).

The `three_gate` F1 carries the same 95% bootstrap CI (B=1000, fixed seed) as §5.2.

| policy | precision | recall | F1 | composed? |
|---|---|---|---|---|
| vanilla_rag | 1.000 | 0.000 | 0.000 | no |
| existence_only | 1.000 | 0.250 | 0.400 | no |
| two_gate (1+2) | 1.000 | 0.500 | 0.667 | yes |
| three_gate (1+2+3) | 0.933 | 0.996 | 0.963 [0.96, 0.97] | yes |

**Additive lift: +56.3 F1 points** — composed validator (three-gate, F1 0.963 [0.96, 0.97]) over the strongest non-composed baseline (F1 0.400). Only `vanilla_rag` and `existence_only` are non-composed (single-rule) policies; in-context-alone and support-alone are not isolable in a nested policy stack. Target was ≥10 points.
