# Reproducibility — one-command artifact evaluation

This guide is the short, scoped reviewer entry point for the ACL
artifact evaluation. It complements `docs/paper/ARTIFACT.md` (the
full appendix) by giving a single environment + a single command
sequence that regenerates every paper artifact, without asking the
reviewer to install Rust, PDFium, or any model cache locally.

If you have already followed `docs/paper/ARTIFACT.md` on a local
toolchain, you do not need this file — it is the **containerized**
reproduction path.

---

## TL;DR — one command

Pick either the Codespaces path or the local Docker path. Both
produce the same environment.

### Option 1 — GitHub Codespaces (zero local install)

1. Open the repo on GitHub.
2. Click **Code → Codespaces → Create codespace on `main`**.
3. The devcontainer (`.devcontainer/devcontainer.json`) builds the
   image, mounts the workspace at `/workspace`, and runs
   `cargo install --path crates/evidence-eval` in `postCreateCommand`
   so `evidence-eval` is on `PATH` when the shell opens.

When the terminal is ready, skip to **Section C — reproduction
sequence** below.

### Option 2 — Local Docker

From the repository root on a machine with Docker installed:

```sh
# Build the pinned image (Debian 12 + Rust 1.85.0). ~5-10 min cold.
docker build -t evidence-eval:repro .

# Drop into a shell with the repo mounted at /workspace.
docker run --rm -it -v "$PWD":/workspace -w /workspace \
    evidence-eval:repro

# Inside the container, install the harness on PATH (one-time).
cargo install --path crates/evidence-eval --force
```

Then run the sequence in **Section C** below.

---

## What the image contains (and does not)

The container is intentionally minimal:

- **Included:** Debian 12 slim, Rust 1.85.0 (matches workspace MSRV
  in `Cargo.toml`, the `rust-toolchain.toml` stable channel, with
  `rustfmt` + `clippy`), `libssl-dev`, `pkg-config`,
  `ca-certificates`, `git`, `curl`, `build-essential`. `rusqlite`
  is built with the `bundled` feature, so no system sqlite is
  required.
- **Not included (fetched at first use):**
  - **PDFium.** The `pdfium-auto` crate downloads the prebuilt
    PDFium binary on the first PDF call. No manual install.
  - **NLI models.** The harness pulls two ONNX checkpoints from
    HuggingFace on the first `--real-nli` / `compare` / `latency`
    call:
    - `Xenova/distilbert-base-uncased-mnli` (~265 MB, baseline).
    - `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`
      (full-precision `model.onnx` at the repo root; ~440 MB).
    - **Budget: ~700 MB of network + disk, one-time. Cached
      afterwards; every subsequent call is offline.**
  - **DailyMed corpus.** Re-fetched in step 0 below via
    `evidence-eval fetch dailymed --limit 50 --output /tmp/corpus`
    (~30-50 MB, public DailyMed v2 API, idempotent on rerun).

Total cold-start network budget: ~700 MB (models) + ~50 MB
(corpus). Total cold-start time: dominated by `cargo build
--release` of the workspace (a few minutes on a modern x86_64
runner) plus the one-time HuggingFace download. After warm-up, the
full reproduction sequence below runs in a few minutes.

---

## Section C — reproduction sequence

This is a one-to-one mirror of `docs/paper/ARTIFACT.md` Section C,
with all paths anchored to `/workspace` (the mount point inside the
container). Run each step from the repo root with `evidence-eval`
on `PATH`. Each step prints the headline number the paper cites;
the expected values are documented inline in `ARTIFACT.md` and the
committed `docs/paper/*.md` outputs.

### 0. Fetch the public corpus (needed for step 6 only)

```sh
evidence-eval fetch dailymed --output ./dailymed-corpus --limit 50
```

### 1. Lint and stat the committed benchmark

```sh
evidence-eval lint  crates/evidence-eval/datasets/fda_label_bench.toml
evidence-eval stats crates/evidence-eval/datasets/fda_label_bench.toml
```

Expect `0 diagnostics`, `254 valid seeds`, materialization preview
`254 → 1,270`.

### 2. Generate the injected set

```sh
evidence-eval inject \
    crates/evidence-eval/datasets/fda_label_bench.toml \
    --output /tmp/inj.toml
```

Expect `wrote 1270 examples to /tmp/inj.toml`.

### 3. §5.2 / §5.3 / claim-3 metrics (with bootstrap CIs)

```sh
evidence-eval metrics /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/rule-metrics.md
```

Headline: existence / in-context F1 = 1.000; support F1 0.929
[0.91, 0.94]; non-overlap 100% / 100%; three-gate F1 0.963 [0.96,
0.97] vs. baseline 0.400 → **+56.3 F1 points**.

### 4. §5.1 NLI model comparison

```sh
evidence-eval compare /tmp/inj.toml \
    --candidate-nli lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/nli-comparison.md
```

Headline: valid retention **167/254 → 181/254 (+14)**; all
**1106/1270 → 1166/1270 (+60)**.

### 5. §5.7 cost / latency

```sh
evidence-eval latency \
    --dataset /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/cost-latency.md
```

Reproducible headline: structural ≈ **2.5%** of wall time, support
(NLI) ≈ **97.5%**.

> **Honest caveat — wall-clock drifts, the split holds.** The
> absolute milliseconds reported in `docs/paper/cost-latency.md`
> (total ≈ 29,105.6 ms, mean ≈ 5.7 ms/call) are machine-dependent
> and will not reproduce exactly inside the container or on a
> different host. Only the **2.5% / 97.5% structural-vs-support
> split** and the per-call counts are treated as reproducible
> headlines. The committed `tools/preflight.sh` script encodes
> this contract — it asserts the split and counts (not the absolute
> ms) — so if `preflight.sh` passes on your reproduction, the
> latency claim has reproduced even if the millisecond column has
> drifted.

### 6. Faithfulness audit

```sh
evidence-eval audit-faithfulness \
    --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
    --corpus ./dailymed-corpus
```

Headline: 254 examples, 255 spans, **249 exact / 1 fuzzy / 5
missing** (6 flagged, all confirmed benign — see
`docs/paper/faithfulness-audit.md`). Exit code 2 is expected (means
`missing` spans were flagged for manual review, all already
documented as faithful reconstruction artifacts).

### 7. Full per-example run (catch matrix)

```sh
evidence-eval run /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output /tmp/run-deberta.md

evidence-eval run /tmp/inj.toml \
    --real-nli \
    --output /tmp/run-distilbert.md
```

Headline catch matrix (DeBERTa three-gate): Valid 181/254 retained,
FabricatedSpan 254/254, OutOfContext 254/254, Unsupported 233/254,
Contradicted 244/254. Distilbert baseline: Valid **167/254**.

### Optional — network-free structural-only reproduction

The existence and in-context rules are model-independent and
reproduce with no model download. Omit `--real-nli`:

```sh
evidence-eval metrics /tmp/inj.toml
evidence-eval run     /tmp/inj.toml
```

Only the existence (1.000) and in-context (1.000) rows are
meaningful without `--real-nli`; the support row is mock and is
**not** a measurement of the NLI model.

---

## Validating your reproduction with `tools/preflight.sh`

After running steps 0-7 you can run the committed preflight script
to mechanically check every reproducible headline in one go:

```sh
EVIDENCE_CORPUS=./dailymed-corpus bash tools/preflight.sh
```

The script re-runs the dataset lint, the faithfulness audit (and
compares the totals to the committed header), the metrics gate, and
the structural-vs-support split — exactly the invariants this guide
promises. A passing `preflight.sh` is the strongest single signal
that the paper numbers have reproduced.

---

## What is out of scope for this artifact

Mirroring `ARTIFACT.md` Section E:

- The benchmark is single-domain (FDA labels) and AI-authored from
  a corrected template; see `fda_label_bench_PROVENANCE.md`.
- The natural-failure / real-LLM study (§5.6) and the ALCE
  comparison (§5.5) are **not** reproduced by this container.
- Absolute latency milliseconds (step 5) are machine-dependent —
  only the **2.5% / 97.5%** split is a reproducible headline.
