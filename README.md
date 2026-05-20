# evidence

[![ci](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml/badge.svg)](https://github.com/Bilalsabry/evidence/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/Bilalsabry/evidence?display_name=tag)](https://github.com/Bilalsabry/evidence/releases)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

A local-first RAG citation validator that decomposes "is this citation correct?" into three independent gates: existence, in-context, and support.

## Thesis

The literature treats RAG citation correctness as a single signal. We argue, and show on the v2 injected FDA-label benchmark, that it decomposes into three independent constraints — **existence** (the cited span ID resolves to a real span in the corpus), **in-context** (that span was actually in the retrieved set the model saw for this query), and **support** (the span entails the claim). The contribution is the decomposition plus the empirical evidence that the three rules catch non-overlapping failure classes on a controlled benchmark; it is not a new model. Conflating the three hides which lever a practitioner should pull when a citation is wrong.

## Headline result

All numbers below are scoped to the **controlled injected FDA-label benchmark, v2, N = 254 seeds → 1,270 examples**. They characterize validator behavior under matched-pair failure injection on a single domain. They are not real-world failure prevalence rates.

| Quantity | Measurement |
|---|---|
| Existence F1 | **1.000** [1.00, 1.00], tp/fp/fn = 254/0/0 |
| In-context F1 | **1.000** [1.00, 1.00], tp/fp/fn = 254/0/0 |
| Support F1 (DeBERTa-v3) | **0.929** [0.91, 0.94], precision 0.873, recall 0.992 |
| Non-overlap | **100%** by the operational blindness measure (OutOfContext accepted by existence-only 254/254; Unsupported+Contradicted accepted by two-gate 508/508) |
| Additive lift over best non-composed baseline | **+56.3 F1 points** (three-gate 0.963 vs. existence-only 0.400) |
| Cost split (5,080 timed calls / pass) | structural **2.5%** / NLI support **97.5%** of wall time |
| Cited-span audit | **zero fabrications**: 249 exact + 1 fuzzy + 5 missing = 6 flagged rows, all manually verified as faithful line/bullet/hyphen/whitespace reconstruction artifacts of genuine label text |

95% CIs are bootstrap (B = 1000, fixed seed) over the 1,270 examples. Existence and in-context are index/string checks and model-independent; the support gate runs an ONNX cross-encoder on CPU.

## Scope

We claim citation **correctness at the validator boundary** — existence, in-context, support — on the v2 injected FDA-label benchmark; we do not claim causal faithfulness (the in-context gate shows a span was available to the model, not that the model relied on it) and we do not claim real-world failure prevalence.

## The paper

**Working title:** *Three Rules for a Citation: Decomposing Faithfulness in Retrieval-Augmented Generation.* Status: draft; arXiv submission pending.

- [`docs/paper/PAPER_EXPLAINER.md`](docs/paper/PAPER_EXPLAINER.md) — the gateway. Self-contained end-to-end explanation; start here.
- [`docs/paper/paper-draft.md`](docs/paper/paper-draft.md) — full Markdown draft.
- [`paper/main.tex`](paper/main.tex) — ACL LaTeX submission package (`acl.sty`, `body.tex`, `related.tex`, `refs.bib`).
- [`docs/paper/CLAIMS.md`](docs/paper/CLAIMS.md) — the measured-not-target claims and the explicit scope statement.

## The artifact

A reviewer reproduces every empirical number from a clean checkout with one install and five commands. The harness is offline after the first model fetch.

```sh
cargo install --path crates/evidence-eval --force
```

```sh
# 1. Fetch the public DailyMed corpus (required only for the faithfulness audit).
evidence-eval fetch dailymed --output ./dailymed-corpus --limit 50

# 2. Inject 254 seeds into 1,270 matched-pair examples.
evidence-eval inject crates/evidence-eval/datasets/fda_label_bench.toml \
    --output /tmp/inj.toml

# 3. Per-rule isolation, non-overlap, additive lift (with bootstrap CIs).
evidence-eval metrics /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/rule-metrics.md

# 4. Structural vs. support cost split.
evidence-eval latency --dataset /tmp/inj.toml \
    --real-nli \
    --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
    --output docs/paper/cost-latency.md

# 5. Cited-span faithfulness audit against the source PDFs.
evidence-eval audit-faithfulness \
    --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
    --corpus ./dailymed-corpus
```

The first `--real-nli` invocation downloads an ONNX MNLI checkpoint from HuggingFace into the local cache. The default `Xenova/distilbert-base-uncased-mnli` is ~265 MB; the headline §5.1 model `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx` is several hundred MB at full precision. Budget for a one-time download of a few hundred MB per model; everything is offline afterward. A network-free structural-only reproduction (existence + in-context only) is available by omitting `--real-nli` — see [`docs/paper/ARTIFACT.md`](docs/paper/ARTIFACT.md) for the full numbered sequence, including the §5.1 NLI comparison and the per-example catch matrix.

## Honest disclosure

The 254 `valid` seeds were produced by an AI-assisted authoring pipeline — large-language-model sub-agents running under a single fixed, published guideline (`docs/EVAL.md`) over a line-level span scaffold extracted by `evidence-eval author` from 50 public-domain DailyMed FDA-label PDFs — and verified by an automated verbatim faithfulness audit (`evidence-eval audit-faithfulness`) that found zero fabrications: 249/255 spans matched exactly, 1 fuzzy, 5 flagged missing, and all six flagged rows were manually located in their source labels and confirmed faithful (whitespace, line-split, bullet-glyph, and hyphen-break extraction artifacts). The pipeline self-corrected once: v1 (263 seeds) shipped a flawed "cite minimally" guideline that produced fragment citations on line-wrapped PDF text; we caught it in our own hand audit, corrected the guideline, re-authored to v2 (254 seeds), and preserved v1 out-of-repo for inspection. A wider human verification pass with an inter-annotator-agreement statistic and a natural-failure / real-LLM prevalence study are explicitly recommended open work, planned per [`docs/paper/SUBMISSION_RUNBOOK.md`](docs/paper/SUBMISSION_RUNBOOK.md) Phase 2 and [`docs/paper/natural-failure-protocol.md`](docs/paper/natural-failure-protocol.md). See [`docs/paper/INTEGRITY.md`](docs/paper/INTEGRITY.md) and [`docs/paper/fda_label_bench_PROVENANCE.md`](docs/paper/fda_label_bench_PROVENANCE.md) for the full disclosure and datasheet.

## Repo map

- [`crates/evidence-core`](crates/evidence-core) — the three-rule validator: existence, in-context, support; the only place where citation correctness is decided.
- [`crates/evidence-eval`](crates/evidence-eval) — the reproduction harness (`run`, `inject`, `metrics`, `compare`, `latency`, `audit-faithfulness`, `fetch`, `lint`, `stats`, `author`) and the committed `fda_label_bench.toml`.
- [`crates/evidence-cli`](crates/evidence-cli) — local CLI surface for the validator.
- [`crates/evidence-api`](crates/evidence-api) — HTTP boundary for embedding the validator.
- [`apps/desktop`](apps/desktop) — desktop client for the local-first workflow.
- [`paper/`](paper) — ACL LaTeX submission package (`main.tex`, `body.tex`, `related.tex`, `refs.bib`).
- [`docs/paper/`](docs/paper) — paper Markdown, claims, datasheet, integrity statement, audits, runbooks. Start at `PAPER_EXPLAINER.md`.
- [`tools/preflight.sh`](tools/preflight.sh) — workspace preflight (build, test, lint).

## License

Apache License 2.0. See [`LICENSE`](LICENSE).

## Cite

```bibtex
@misc{evidence2026,
  title  = {Three Rules for a Citation: Decomposing Faithfulness in
            Retrieval-Augmented Generation},
  author = {Sabry, Bilal},
  year   = {2026},
  note   = {arXiv submission pending --- replace on post},
  howpublished = {\url{https://github.com/Bilalsabry/evidence}}
}
```
