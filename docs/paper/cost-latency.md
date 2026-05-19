# Cost & latency (§5.7)

Harness-level decomposition over **1270** examples (5080 timed (example, policy) calls per pass). NLI checkpoint: `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`.

| stage | total | mean / call | share |
|---|---|---|---|
| structural (existence + in-context + harness) | 713.7 ms | 140.5 µs | 2.5% |
| support gate (NLI forward passes) | 28391.8 ms | 5.6 ms | 97.5% |
| **total (real-NLI run)** | **29105.6 ms** | **5.7 ms** | **100%** |

**Structural vs. support split:** 2.5% structural / 97.5% support, by wall time of the real-NLI run.

> Measurement method (honest note): per-gate cost is measured at the harness level, not by instrumenting `evidence-core`. The structural figure is the wall time of a mock-support run (the support checker is an in-memory class-derived stub, ≈0 cost); it bundles the existence + in-context gates with harness setup (in-memory SQLite seeding, prompt/answer construction). The support figure is the *delta* of a real-NLI run over that mock run on the same dataset, so it folds tokenization, the NLI forward pass, and verdict mapping into one number, and includes a one-time model warm-up on the first support call. Existence vs. in-context are not separated — both are O(µs) index/string checks dwarfed by the NLI pass. Treat these as order-of-magnitude, not microbenchmark, figures.
