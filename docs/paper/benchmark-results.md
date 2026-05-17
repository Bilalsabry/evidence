# Benchmark results — FDA drug-label set (N = 263 seeds)

First full run on the real FDA-label benchmark (`fda_label_bench.toml`,
263 `valid` seeds → 1,315 examples via `inject` → 5,260 (example,
policy) rows). Support gate: `distilbert-base-uncased-mnli` via
`run --real-nli`. See `fda_label_bench_PROVENANCE.md` for how the
seeds were produced and audited (AI-authored; spot-checked, not
exhaustively human-verified).

**Overall agreement: 4,990 / 5,260 (94.9%).**

## Catch matrix (`accepted / refused`)

| class \ policy | vanilla | existence | two-gate | three-gate |
|---|---|---|---|---|
| Valid | 263 / 0 | 263 / 0 | 263 / 0 | **134 / 129** |
| FabricatedSpan | 263 / 0 | **0 / 263** | 0 / 263 | 0 / 263 |
| OutOfContext | 263 / 0 | 263 / 0 | **0 / 263** | 0 / 263 |
| Unsupported | 263 / 0 | 263 / 0 | 263 / 0 | **4 / 259** |
| Contradicted | 263 / 0 | 263 / 0 | 263 / 0 | **17 / 246** |

## Reading it

**Rules 1 & 2 — existence and in-context — are flawless at scale.**

- FabricatedSpan: caught 263/263 at the existence gate, and *only* at
  the existence gate (vanilla accepts all; existence/two/three all
  refuse all). Zero leakage.
- OutOfContext: caught 263/263 at the in-context gate (two-gate and
  three-gate refuse all; existence-only correctly does *not* — it is a
  different rule). Zero leakage.

The kernel claim — these are independent gates that catch disjoint
error classes — holds perfectly on 263 real drug-label examples, not
just the synthetic fixture. This is the §5.2/§5.3 result.

**Rule 3 — support — catches genuine failures, over-refuses valid.**

- Recall on real failures is high: Contradicted 246/263 (93.5%),
  Unsupported 259/263 (98.5%). The NLI gate does its job on bad
  citations.
- But valid-retention is poor: **129/263 (49%) of true,
  correctly-cited claims are false-refused** at three-gate under
  distilbert. The remaining disagreements are reason-level mismatches
  (NLI refusing an `unsupported` example as `contradicted` or vice
  versa) — still a refusal, wrong label.

## Why this strengthens the paper

The failure mode is **false-refusal-on-valid, never missed-failure**.
The structural gates are exact; the weak NLI is conservative to a
fault. This is the §5.1 argument made at scale: the support gate must
be measured *independently* of the structural rules (mock-mode is
100% by construction; only the real-NLI column moves), and a
49%-false-refusal distilbert is concrete motivation for the
DeBERTa-v3-large upgrade. The open question — does the false-refusal
rate collapse under a stronger model — is answerable in one command:

```sh
# inject first (compare runs the dataset as-is; it does NOT inject),
# then compare distilbert vs DeBERTa on the full failure corpus.
evidence-eval inject crates/evidence-eval/datasets/fda_label_bench.toml \
    --output /tmp/fda_injected.toml
evidence-eval compare /tmp/fda_injected.toml \
    --candidate-nli lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/nli-comparison.md
```

## Caveats (do not over-read)

- Seeds are AI-authored from a small set of templates/rules; more
  homogeneous than an independently hand-curated set. Effect-size
  reads should weigh this. See the datasheet.
- `lint` is structural; only a sample was audited against source PDFs.
- Single NLI checkpoint, single retrieval config, no latency numbers,
  no inter-annotator agreement.
