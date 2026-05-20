# Reproducibility Verification — `docs/paper/ARTIFACT.md`

This document records a fresh, end-to-end execution of every command in
`docs/paper/ARTIFACT.md` against a clean working tree, and the diff of
each regenerated artifact against the canonical copy committed under
`docs/paper/`. It is integrity evidence for reviewers: the headline
numbers in the paper are derived solely from these commands, and the
files in this repository are the unaltered output of those commands.

## Run metadata

| Field | Value |
|---|---|
| Run timestamp (UTC) | `2026-05-20T21:10:31Z` |
| Host OS | Darwin 23.5.0 (macOS 14.5), arm64 |
| Repo path | `/Users/bilalsabry/code/evidence` |
| Git commit | `ba1bb19b5d4d7d1b234c45eac33bf3ca5f479ec7` |
| Git status before run | clean (`git status` → "nothing to commit, working tree clean") |
| Rust toolchain | stable (pinned by `rust-toolchain.toml`) |
| `evidence-eval` install | `cargo install --path crates/evidence-eval --force` → `evidence-eval v0.0.1` |
| NLI candidate model | `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx` (HuggingFace cache hit, no network) |
| NLI baseline model | `Xenova/distilbert-base-uncased-mnli` (HuggingFace cache hit, no network) |
| Corpus path (step 6) | `~/dailymed-corpus/` (manifest already present from prior fetch) |

All regenerated outputs were written under `/tmp/repro-*` so no
committed file in `docs/paper/` was modified by the verification run.

## Commands executed (verbatim)

```sh
# Step 0 — corpus fetch skipped (already on disk from prior run; the
# committed dataset is self-contained for steps 1–5 and 7).

# Build / install
cargo install --path crates/evidence-eval --force

# Step 1 — preflight (stdout only, no --output)
evidence-eval lint  crates/evidence-eval/datasets/fda_label_bench.toml
evidence-eval stats crates/evidence-eval/datasets/fda_label_bench.toml

# Step 2 — inject
evidence-eval inject \
    crates/evidence-eval/datasets/fda_label_bench.toml \
    --output /tmp/inj.toml

# Step 3 — §5.2 / §5.3 / claim 3 metrics
evidence-eval metrics /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output /tmp/repro-rule-metrics.md

# Step 4 — §5.1 NLI model comparison
evidence-eval compare /tmp/inj.toml \
    --candidate-nli lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output /tmp/repro-nli-comparison.md

# Step 5 — §5.7 cost / latency
evidence-eval latency \
    --dataset /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output /tmp/repro-cost-latency.md

# Step 6 — faithfulness audit (prints to stdout)
evidence-eval audit-faithfulness \
    --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
    --corpus ~/dailymed-corpus \
    > /tmp/repro-faithfulness-audit.md

# Step 7 — full per-example run, both NLI checkpoints
evidence-eval run /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output /tmp/repro-run-deberta.md

evidence-eval run /tmp/inj.toml --real-nli --output /tmp/repro-run-distilbert.md
```

## Preflight (Step 1) — stdout assertions

`lint` reported `0 diagnostics. Dataset is clean.` (exit 0). `stats`
reported 254 `valid` seeds and the materialization projection
`valid seeds: 254 → grand total after inject: 1270`. Both match the
ARTIFACT preflight expectation.

## Inject (Step 2) — stderr assertion

`inject` wrote `wrote 1270 examples to /tmp/inj.toml (254 seeds + 1016
injected; 0 non-valid skipped)`, matching the ARTIFACT line verbatim.

## Artifact diff verdicts

Each row pairs a canonical file under `docs/paper/` with its
freshly-regenerated `/tmp/repro-…` counterpart and runs `diff -u`.

| # | Canonical artifact | Regenerated copy | Verdict | Note |
|---|---|---|---|---|
| 3 | `docs/paper/rule-metrics.md` | `/tmp/repro-rule-metrics.md` | **BIT-IDENTICAL** | SHA-256 `9bfb9bbc…0311c8f9` on both files. §5.2 per-rule numbers, §5.3 non-overlap, and the claim-3 +56.3 F1 lift all reproduce exactly. |
| 4 | `docs/paper/nli-comparison.md` | `/tmp/repro-nli-comparison.md` | **BIT-IDENTICAL** | SHA-256 `b4044f86…23a6c2a8d8` on both files. §5.1 valid-retention 167/254 → 181/254 (+14) and the per-class agreement table reproduce exactly. |
| 5 | `docs/paper/cost-latency.md` | `/tmp/repro-cost-latency.md` | **NUMERIC-MATCH-FORMAT-DRIFT** | Wall-clock milliseconds differ (total 29,105.6 ms canonical vs. 46,043.0 ms on this host) and the structural/support split moved 2.5% / 97.5% → 2.2% / 97.8%. The ARTIFACT explicitly flags absolute milliseconds as machine-dependent. The reproducible headline — *structural is a low single-digit percent of wall time, NLI dominates at ≈98%* — holds. Honest drift, not a regression. |
| 6 | `docs/paper/faithfulness-audit.md` | `/tmp/repro-faithfulness-audit.md` | **BIT-IDENTICAL (tool output)** | The auto-generated portion (totals + flagged-spans table) is byte-identical: examples 254, corpus spans 255, exact 249, fuzzy 1, missing 5; the six flagged rows match by example name. Exit code 2 as expected. The canonical file additionally contains a manually-authored "Manual verification of flagged spans (2026-05-18)" appendix that the binary does not emit; that section is documentation, not a tool output, and is correctly absent from the regenerated file. |
| 7a | (no canonical file — `run` output is intermediate) | `/tmp/repro-run-deberta.md` | **MATCHES PUBLISHED CATCH MATRIX** | Three-gate column: Valid 181 / 73, FabricatedSpan 0 / 254, OutOfContext 0 / 254, Unsupported 3 / 251, Contradicted 1 / 253. Valid retention 181/254 matches the §5.1 catch-matrix headline in `benchmark-results.md` exactly; FabricatedSpan and OutOfContext refuse 254/254 as claimed. Harness reported `Agreement: 4976 / 5080 rows match the expected outcome matrix`. |
| 7b | (no canonical file) | `/tmp/repro-run-distilbert.md` | **MATCHES PUBLISHED CATCH MATRIX** | Distilbert-baseline run; Valid 167/254 retained, matching the ARTIFACT step-7 expectation. Harness reported `Agreement: 4916 / 5080 rows match the expected outcome matrix`. |

### Claims → verdict (mirrors ARTIFACT §D)

| Paper claim | Source (regenerated file) | Verdict |
|---|---|---|
| §5.2 existence: precision/recall/F1 = 1.000, tp/fp/fn 254/0/0 | `repro-rule-metrics.md` | **Reproduces exactly** |
| §5.2 in-context: precision/recall/F1 = 1.000, tp/fp/fn 254/0/0 | `repro-rule-metrics.md` | **Reproduces exactly** |
| §5.2 support: P=0.873, R=0.992, F1=0.929 [0.91, 0.94], tp/fp/fn 504/73/4 | `repro-rule-metrics.md` | **Reproduces exactly** |
| §5.3 OutOfContext accepted by existence-only 254/254 (100.0%) | `repro-rule-metrics.md` | **Reproduces exactly** |
| §5.3 Unsupported+Contradicted accepted by two-gate 508/508 (100.0%) | `repro-rule-metrics.md` | **Reproduces exactly** |
| Claim 3 — three-gate F1 0.963 vs 0.400 baseline → +56.3 points | `repro-rule-metrics.md` | **Reproduces exactly** |
| §5.1 — valid retention 167/254 → 181/254 (+14); all 1106/1270 → 1166/1270 (+60) | `repro-nli-comparison.md` | **Reproduces exactly** |
| §5.7 — structural ≪ support cost split | `repro-cost-latency.md` | **Reproduces qualitatively (2.2% / 97.8% vs. canonical 2.5% / 97.5%; absolute ms machine-dependent — flagged in ARTIFACT)** |
| Faithfulness audit — 254 examples, 255 spans, 249 / 1 / 5, 6 flagged rows | `repro-faithfulness-audit.md` | **Reproduces exactly (tool output bit-identical)** |
| §5.1 catch matrix — DeBERTa Valid 181/254; distilbert Valid 167/254 | `repro-run-deberta.md`, `repro-run-distilbert.md` | **Reproduces exactly** |

## Honest notes on observed drift

- **`cost-latency.md` is the only artifact with numeric drift.** Total
  wall time on this host (Apple M-series, macOS 14.5, arm64) was
  ≈46 s vs. ≈29 s on the canonical-generation host; structural cost
  also scaled with it, leaving the structural-vs-support split at
  2.2% / 97.8% instead of 2.5% / 97.5%. Both clocks tell the same
  story: structural gates are negligible (low single-digit percent),
  NLI dominates. The ARTIFACT explicitly designates absolute
  milliseconds as machine-dependent and the structural-vs-support
  split as the reproducible headline; the split is preserved to one
  significant figure.
- **`faithfulness-audit.md` drift is documentation, not output.** The
  binary's stdout (header, totals table, flagged-spans table) is
  byte-identical to the corresponding portion of the canonical file.
  The canonical file additionally contains a hand-authored
  "Manual verification of flagged spans" section dated 2026-05-18
  documenting verbatim PDF lookups for each flagged span; this is not
  emitted by `audit-faithfulness` and is correctly absent from the
  regenerated copy.
- **No other artifact drifted.** `rule-metrics.md` and
  `nli-comparison.md` are SHA-256-identical to the committed files;
  the `run` output catch matrices match the published ones row-for-row.

## Bottom line

**All headline numbers reproduce.** Two of the four committed paper
artifacts are byte-for-byte identical to a fresh regeneration; the
faithfulness-audit tool output is byte-for-byte identical (the
canonical file's extra section is hand-authored documentation, not
tool output); the cost-latency file shows machine-dependent
millisecond drift that the ARTIFACT explicitly anticipates, with the
designated reproducible headline (structural ≈ low single-digit
percent of wall time, NLI ≈ the rest) preserved.
