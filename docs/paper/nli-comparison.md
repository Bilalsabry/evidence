# NLI model comparison (§5.1) — three-gate

- **baseline:** `distilbert (default)`
- **candidate:** `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`

Per-class agreement at the `three_gate` policy (the only policy whose verdict depends on the NLI model). Δ is candidate − baseline in agreed rows.

| class | distilbert (default) | lquint/DeBERTa-v3-base-mnli-fever-anli-onnx | Δ |
|---|---|---|---|
| valid | 167 / 254 | 181 / 254 | +14 |
| fabricated_span | 254 / 254 | 254 / 254 | 0 |
| out_of_context | 254 / 254 | 254 / 254 | 0 |
| unsupported | 219 / 254 | 233 / 254 | +14 |
| contradicted | 212 / 254 | 244 / 254 | +32 |
| **all** | **1106 / 1270** | **1166 / 1270** | **+60** |

**Headline (valid-retention):** 167 / 254 → 181 / 254 (+14 false-refusals recovered). This is the §5.1 payload — does the stronger model stop refusing true claims?
