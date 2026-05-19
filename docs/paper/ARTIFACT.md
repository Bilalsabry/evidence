# Artifact Appendix — Reproduction Guide

This appendix lets a reviewer regenerate every empirical number in the
paper from a clean checkout. The artifact is the `evidence-eval`
harness plus the committed FDA-label benchmark; no proprietary data,
network credentials, or GPU is required. The public corpus is fetched
from the DailyMed v2 API at audit time.

All findings are produced on the controlled v2 injected FDA-label
benchmark. They characterize validator behavior under matched-pair
injected failures on a single domain; they are not real-world failure
prevalence rates.

---

## A. Software environment

| Component | Requirement |
|---|---|
| OS | Linux or macOS (x86_64 or arm64). No GPU required — NLI runs on CPU. |
| Rust toolchain | `rust-toolchain.toml` pins `channel = "stable"` with `rustfmt` + `clippy`. A recent stable `rustup` toolchain installs automatically on first `cargo` invocation in the repo. |
| Disk | ~2 GB: build artifacts, the fetched DailyMed corpus, and the ONNX NLI models. |
| Network | Required once for the corpus fetch (`fetch dailymed`, public DailyMed v2 API) and once for the first-run HuggingFace model download. The harness is otherwise offline. |
| Embedding model | `bge-small-en-v1.5` auto-downloads via `fastembed` for ingest paths; not exercised by the harness commands below. |

### Build and install

```sh
# 1. From the repository root.
cd evidence

# 2. Build + test the workspace (sanity check; optional but recommended).
cargo build --workspace
cargo test --workspace --all-targets

# 3. Install the eval harness binary onto PATH.
cargo install --path crates/evidence-eval --force
```

After step 3, `evidence-eval --help` lists the ten subcommands:
`run`, `inject`, `lint`, `stats`, `author`, `fetch`, `compare`,
`metrics`, `latency`, `audit-faithfulness`.

### First-run model download

The first command that passes `--real-nli` (or any `compare` /
`latency` invocation) downloads an ONNX MNLI checkpoint from
HuggingFace into the local cache:

- Default support model: `Xenova/distilbert-base-uncased-mnli`
  (~265 MB).
- §5.1 candidate / §5.7 model:
  `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx` (full-precision
  `model.onnx` at the repo root; several hundred MB).

Budget for a one-time download of a few hundred MB per model. After
the first run the models are cached and every later command is
offline.

### Determinism caveats

- **NLI on CPU.** The support gate runs the ONNX cross-encoder on CPU.
  Verdicts are deterministic for a fixed model; cross-machine
  floating-point differences are below the decision threshold for this
  benchmark, so per-class catch counts reproduce exactly.
- **Bootstrap CIs use a fixed seed.** The 95% bootstrap intervals in
  §5.2 / §5.3 / claim-3 use B = 1000 resamples-with-replacement over
  the 1270 examples with a fixed seed, so the reported intervals
  reproduce exactly.
- **Structural rules need no model.** Existence and in-context are
  index/string checks and are model-independent. Their results
  reproduce in mock mode (omit `--real-nli`) with no model download —
  useful for a network-free structural-only reproduction.
- **Corpus fetch is idempotent.** `fetch dailymed` writes a
  `manifest.toml` with stable set IDs and SHA-256 hashes; reruns skip
  files already on disk that match the manifest. The committed
  benchmark already encodes its spans, so a fresh fetch is only needed
  for the faithfulness audit (which re-reads the source PDFs).

---

## B. Inputs

| Input | Path / source | Role |
|---|---|---|
| Committed benchmark | `crates/evidence-eval/datasets/fda_label_bench.toml` | 254 hand-shaped `valid` seeds from 50 public-domain DailyMed FDA labels. Provenance: `docs/paper/fda_label_bench_PROVENANCE.md`. |
| Injected set | generated below into `/tmp/inj.toml` | 254 seeds → 1,270 matched-pair examples (one existence + one in-context + support-mutation variants per seed). |
| Public corpus | `evidence-eval fetch dailymed` → `./dailymed-corpus/` | The source PDFs, needed only by `audit-faithfulness`. |
| Bootstrap dataset | `crates/evidence-eval/datasets/bootstrap.toml` | Small wiring/regression set; not a source of paper headline numbers. |

---

## C. Numbered reproduction sequence

Run from the repository root with `evidence-eval` on PATH (Section A,
step 3). Each step states the exact headline output to expect.

### 0. Fetch the public corpus (needed for step 6 only)

```sh
evidence-eval fetch dailymed --output ./dailymed-corpus --limit 50
```

Writes `./dailymed-corpus/manifest.toml` and
`./dailymed-corpus/labels/*.pdf`. Expected: `fetched 50 labels into
./dailymed-corpus` (idempotent on rerun). Steps 1–5 and 7 do not need
the corpus on disk — the benchmark TOML is self-contained.

### 1. Lint and stat the committed benchmark (preflight)

```sh
evidence-eval lint  crates/evidence-eval/datasets/fda_label_bench.toml
evidence-eval stats crates/evidence-eval/datasets/fda_label_bench.toml
```

Expected: whole-file `lint` reports **0 diagnostics** (exit 0).
`stats` reports **254 `valid` seeds** and the materialization
preview (254 → 1,270 after `inject`).

### 2. Generate the injected set

```sh
evidence-eval inject \
    crates/evidence-eval/datasets/fda_label_bench.toml \
    --output /tmp/inj.toml
```

Expected stderr: `wrote 1270 examples to /tmp/inj.toml (254 seeds +
1016 injected; 0 non-valid skipped)`. (254 seeds + 1,016 injected
variants = 1,270 examples → 5,080 (example, policy) rows.)

### 3. §5.2 / §5.3 / claim-3 metrics, with bootstrap CIs

```sh
evidence-eval metrics /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/rule-metrics.md
```

Expected headline output (matches `docs/paper/rule-metrics.md`):

- §5.2 per-rule isolation:
  - existence (owns `fabricated_span`): precision **1.000**, recall
    **1.000**, F1 **1.000 [1.00, 1.00]**, tp/fp/fn = 254/0/0.
  - in-context (owns `out_of_context`): precision **1.000**, recall
    **1.000**, F1 **1.000 [1.00, 1.00]**, tp/fp/fn = 254/0/0.
  - support (owns `unsupported`+`contradicted`): precision 0.873,
    recall **0.992**, F1 **0.929 [0.91, 0.94]**, tp/fp/fn = 504/73/4.
- §5.3 non-overlap (disjoint error classes), by the operational
  blindness measure:
  - OutOfContext accepted by existence-only: **254/254 (100.0%)**.
  - Unsupported+Contradicted accepted by two-gate: **508/508
    (100.0%)**.
- Claim 3 — additive lift: three-gate F1 **0.963 [0.96, 0.97]** vs.
  strongest non-composed baseline (`existence_only`) F1 **0.400** →
  **+56.3 F1 points** (target was ≥10).

### 4. §5.1 NLI model comparison

```sh
evidence-eval compare /tmp/inj.toml \
    --candidate-nli lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/nli-comparison.md
```

Expected headline output (matches `docs/paper/nli-comparison.md`),
per-class three-gate agreement, distilbert (baseline) vs. DeBERTa
(candidate):

| class | distilbert | DeBERTa | Δ |
|---|---|---|---|
| valid | 167 / 254 | 181 / 254 | +14 |
| fabricated_span | 254 / 254 | 254 / 254 | 0 |
| out_of_context | 254 / 254 | 254 / 254 | 0 |
| unsupported | 219 / 254 | 233 / 254 | +14 |
| contradicted | 212 / 254 | 244 / 254 | +32 |
| **all** | **1106 / 1270** | **1166 / 1270** | **+60** |

Headline (valid-retention): **167 / 254 → 181 / 254 (+14
false-refusals recovered)** — the §5.1 payload.

### 5. §5.7 cost / latency

```sh
evidence-eval latency \
    --dataset /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/cost-latency.md
```

Expected headline output (matches `docs/paper/cost-latency.md`), over
1,270 examples / 5,080 timed calls per pass: structural (existence +
in-context + harness) ≈ **2.5%** of wall time; support gate (NLI
forward passes) ≈ **97.5%**. Absolute milliseconds are
order-of-magnitude, machine-dependent figures (the doc reports total
≈ 29,105.6 ms, mean ≈ 5.7 ms/call); the **2.5% / 97.5%** split is the
reproducible structural-vs-support headline.

### 6. Faithfulness audit

```sh
evidence-eval audit-faithfulness \
    --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
    --corpus ./dailymed-corpus
```

Expected headline output (matches
`docs/paper/faithfulness-audit.md`): examples **254**, corpus spans
**255**, **249 exact**, **1 fuzzy**, **5 missing** (5 flagged
`missing` + 1 `noxivent_indication` flagged `fuzzy` = **6 flagged
rows** total). All six flagged spans are confirmed faithful
line/bullet/hyphen/whitespace reconstruction artifacts of genuine
label text (see the manual-verification section of
`faithfulness-audit.md`); none are fabricated. The command exits 2
because `missing` rows are present — this is expected for this
benchmark and does not indicate a fabricated span.

### 7. Full per-example run (catch matrix)

```sh
evidence-eval run /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output /tmp/run-deberta.md
```

Expected catch matrix (DeBERTa three-gate column; matches the §5.1
table in `docs/paper/benchmark-results.md`): Valid 181/254 retained,
FabricatedSpan refused at existence (254/254), OutOfContext refused at
in-context (254/254), Unsupported 233/254 refused, Contradicted
244/254 refused. For the distilbert baseline catch matrix (Valid
**167/254** retained), rerun without `--nli-model`:

```sh
evidence-eval run /tmp/inj.toml --real-nli --output /tmp/run-distilbert.md
```

### Network-free structural-only reproduction (optional)

Structural rules are model-independent and reproduce with no model
download. Omit `--real-nli`:

```sh
evidence-eval metrics /tmp/inj.toml          # mock support; existence/in-context exact
evidence-eval run     /tmp/inj.toml          # structural rows reproduce
```

In mock mode the support row is 100% by construction (the mock checker
is class-derived) and is **not** a measurement of the NLI model —
only the existence (1.000) and in-context (1.000) rows are meaningful
without `--real-nli`.

---

## D. Claims → command → expected output

| Paper claim | Command (Section C step) | Expected headline output |
|---|---|---|
| §5.2 — existence rule is exact | Step 3 (`metrics --real-nli --nli-model …DeBERTa…`) | existence precision/recall/F1 = 1.000, F1 95% CI [1.00, 1.00], tp/fp/fn 254/0/0 |
| §5.2 — in-context rule is exact | Step 3 | in-context precision/recall/F1 = 1.000, F1 95% CI [1.00, 1.00], tp/fp/fn 254/0/0 |
| §5.2 — support gate is high-recall | Step 3 | support precision 0.873, recall 0.992, F1 **0.929 [0.91, 0.94]**, tp/fp/fn 504/73/4 |
| §5.3 — disjoint error classes (100% non-overlap) | Step 3 | OutOfContext accepted by existence-only 254/254 (**100.0%**); Unsupported+Contradicted accepted by two-gate 508/508 (**100.0%**) |
| Claim 3 — additive lift +56.3 | Step 3 | three-gate F1 **0.963 [0.96, 0.97]** vs. baseline 0.400 → **+56.3 points** |
| §5.1 — stronger NLI recovers false-refusals | Step 4 (`compare`) | valid retention **167/254 → 181/254 (+14)**; all **1106/1270 → 1166/1270 (+60)** |
| §5.7 — structural vs support cost split | Step 5 (`latency`) | structural **2.5%** / support (NLI) **97.5%** of wall time |
| Benchmark faithfulness (no fabricated spans) | Step 6 (`audit-faithfulness`) | 254 examples, 255 spans, **249 exact / 1 fuzzy / 5 missing** (6 flagged, all confirmed benign) |
| §5.1 catch matrix (per-example) | Step 7 (`run --real-nli --nli-model …`) | DeBERTa three-gate Valid **181/254**; distilbert (no `--nli-model`) Valid **167/254**; FabricatedSpan / OutOfContext 254/254 |

---

## E. Notes and limitations

- The benchmark is single-domain (FDA labels); generalization beyond
  it is unmeasured. Seeds are AI-authored from a corrected
  template/guideline and are more homogeneous than independent hand
  curation (see `fda_label_bench_PROVENANCE.md` and the
  benchmark-results datasheet).
- The natural-failure / real-LLM study (§5.6) and the ALCE comparison
  (§5.5) are explicitly out of scope and not reproduced by this
  artifact.
- Absolute latency milliseconds (Section C step 5) are
  machine-dependent. Only the **2.5% / 97.5%** structural-vs-support
  split is treated as a reproducible headline; per-gate cost is
  measured at the harness level (mock-pass wall time for structural;
  the real-NLI delta for support), not by instrumenting
  `evidence-core`.
