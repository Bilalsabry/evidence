# Support-gate aggregation ablation (§3.4)

The support gate checks whether the cited evidence entails the response
sentence. When a sentence cites *more than one* span, the per-span
verdicts must be combined. We implement and compare two policies
(`--support-agg strict|concat`):

- **Per-span strict-wins** (default): NLI each cited span against the
  sentence independently; the worst verdict wins (any `contradicts` →
  contradicted, else any `neutral` → unsupported, else supported).
  "Every citation must hold on its own."
- **Concatenated-evidence**: join all cited spans into one premise and
  make a single NLI call. "The union of citations may support a claim
  no single span supports."

## Result on the v2 benchmark: the two are identical

Run under `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx` over the 1,270
injected examples, the support rule is **byte-identical** under both
policies:

| aggregation | support precision | support recall | support F1 | tp/fp/fn |
|---|---|---|---|---|
| strict-wins | 0.873 | 0.992 | 0.929 [0.91, 0.94] | 504/73/4 |
| concatenated | 0.873 | 0.992 | 0.929 [0.91, 0.94] | 504/73/4 |

**Why:** every `valid` seed in the v2 benchmark cites exactly **one**
span (254/254; zero multi-span citations — a consequence of the
corrected authoring guideline that requires the cited span to be a
single complete sentence, `docs/EVAL.md`). With a single cited span,
"join the spans" is the identity, so the two policies issue the same
NLI call and return the same verdict.

## What this means

- For **single-span-cited** claims — the clean, common case — the
  strict-wins-vs-concatenated design question is **moot**. The reported
  §5.1–§5.4 numbers are invariant to the aggregation choice. This
  removes a potential reviewer concern: our headline support numbers do
  not depend on an arguable aggregation decision.
- The two policies diverge **only** for multi-span citations, where a
  claim is supported by the union of spans but by no single span. The
  v1 bootstrap contained such cases (e.g. `valid_two_spans_one_sentence`,
  where strict-wins false-refused a genuinely-supported sentence); the
  v2 benchmark deliberately does not, because the corrected guideline
  cites one complete sentence per claim.
- We keep **strict-wins** as the default (the stronger safety claim:
  every citation holds on its own) and ship concatenated-evidence as an
  opt-in for multi-span settings. A multi-span benchmark that exercises
  the divergence is future work.

Reproduce:
```sh
evidence-eval inject crates/evidence-eval/datasets/fda_label_bench.toml --output /tmp/inj.toml
evidence-eval metrics /tmp/inj.toml --real-nli \
  --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx --support-agg strict
evidence-eval metrics /tmp/inj.toml --real-nli \
  --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx --support-agg concat
```
