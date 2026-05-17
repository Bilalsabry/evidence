# Benchmark provenance (datasheet) — v2

**What this is:** a 254-seed citation-validation benchmark derived from
50 public FDA drug-label PDFs (DailyMed, public domain), expanding to
1,270 labeled examples via the `evidence-eval inject` matched-pair
operators.

**How it was generated (be precise — this matters for the paper):**

- Source corpus: `evidence-eval fetch dailymed --limit 50` (DailyMed
  v2 API, public-domain SPL labels). Manifest with SHA-256 per file.
- The `valid` seed examples were **AI-authored** by Claude (Anthropic)
  via parallel sub-agents, driven by the `evidence-eval author` span
  scaffold over each PDF. Each example: one cited true sentence + two
  contrast-set `support_mutations` (one `contradicted`, one
  `unsupported`).
- Authoring constraints (v2): the cited span must be **one complete,
  self-contained sentence** — wrapped PDF lines from the line-level
  `author` output concatenated into a single span (single-space join,
  CRLF trimmed, verbatim words; no paraphrase or invention). Response
  sentence is a faithful restatement entailed by that span alone. Two
  typed mutations with varied mechanisms (number flip / negation /
  entity swap / polarity reversal).
- Every file passed `evidence-eval lint`; whole-directory lint: 0
  diagnostics.

**Audit history (kept on purpose):**

- **v1 (263 seeds)** used a flawed guideline ("cite minimally, one
  span"). On line-wrapped PDF text this produced *fragment* citations
  (e.g. cited `"Doxycycline is virtually completely"` with *absorbed*
  in the next, uncited line). A hand audit of v1's false-refused
  `valid` examples against source PDFs found this was a
  benchmark-authoring bug, not an NLI finding — it inflated the
  false-refusal rate by ~15 points (49% → ~34% real).
- **v2** corrected the guideline (`docs/EVAL.md`) and re-authored all
  seeds with complete-sentence cited spans. v2 faithfulness was
  spot-audited (amiodarone, dapagliflozin, doxycycline): merged spans
  are complete sentences, verbatim from source.
- v1 is preserved at `~/evidence-bench-v1/` (not committed).

**Known limitations a reader/reviewer should weigh:**

- Seeds are AI-authored from a corrected template/guideline; more
  homogeneous than an independently hand-curated benchmark.
- `lint` verifies structure, not semantic entailment. v2 entailment
  was spot-audited (3 labels) plus the full v1 false-refusal audit;
  not an exhaustive human pass over all 254 seeds.
- Single retrieval config; one candidate NLI checkpoint
  (`lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`); no inter-annotator
  agreement.

**Recommended use:** a strong scaffold benchmark with a documented
audit trail. For publication, a wider human verification pass over the
seeds and disclosure of the AI-assisted authoring method is the
conservative path.
