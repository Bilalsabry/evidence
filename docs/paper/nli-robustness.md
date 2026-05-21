# NLI checkpoint robustness (§5.1) — the gate behavior is not a single-model artifact

The §5.1 finding (`nli-comparison.md`) rests on two checkpoints: the default
`Xenova/distilbert-base-uncased-mnli` and `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`.
To show the support gate's qualitative behavior — high recall on injected failures,
conservative valid-precision (it over-refuses some true claims) — survives a change of
NLI backbone, we re-ran the same injected set under two additional MNLI ONNX checkpoints
spanning distinct architectures (RoBERTa-large, DeBERTa-v3-small).

All four models share the same 1270-example injected set
(`evidence-eval inject` over `fda_label_bench.toml`: 254 valid seeds + 1016 injected
variants). The structural rows are computed with the deterministic structural gates and
are model-independent by construction; the support row and valid-retention come from a
real-NLI pass under each checkpoint.

## Per-model results

| model | arch | valid-retention (n/254) | support precision | support recall | support F1 | structural F1 (existence / in-context) |
|---|---|---|---|---|---|---|
| `Xenova/distilbert-base-uncased-mnli` (default) | DistilBERT | 167 / 254 | 0.851 | 0.978 | 0.910 | 1.000 / 1.000 |
| `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx` | DeBERTa-v3-base | 181 / 254 | 0.873 | 0.992 | 0.929 | 1.000 / 1.000 |
| `Xenova/roberta-large-mnli` | RoBERTa-large | 177 / 254 | 0.867 | 0.992 | 0.926 | 1.000 / 1.000 |
| `Xenova/nli-deberta-v3-small` | DeBERTa-v3-small | 169 / 254 | 0.854 | 0.982 | 0.914 | 1.000 / 1.000 |

Sources (all real runs over the same `/tmp/inj.toml`): valid-retention for distilbert and
DeBERTa-v3-base from `nli-comparison.md`; DeBERTa-v3-base support row from `rule-metrics.md`
(generated under that checkpoint); distilbert support row from a fresh
`evidence-eval metrics --real-nli` (default model) run; RoBERTa-large and DeBERTa-v3-small
from fresh `evidence-eval metrics --real-nli --nli-model <repo>` runs. No cell is fabricated;
every number is a measured `evidence-eval` output.

## Trend

Across all four checkpoints the structural gates are exact and identical — existence and
in-context both score F1 1.000 (254/254 fabricated-span and out-of-context caught), confirming
those rules are model-independent. The support gate keeps high recall on real injected failures
under every model (0.978–0.992), so the validator's ability to catch unsupported and contradicted
claims does not hinge on the default checkpoint. Valid-retention and support precision do vary
with the backbone (valid-retention 167–181 / 254; support precision 0.851–0.873), but every model
stays on the same conservative side of the trade-off: each over-refuses a similar minority of
genuinely valid claims rather than leaking failures. The §5.1 result is therefore a property of
the three-gate composition, not an artifact of one NLI checkpoint.
