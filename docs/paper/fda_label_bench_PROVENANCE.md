# Benchmark provenance (datasheet)

**What this is:** a 263-seed citation-validation benchmark derived from
50 public FDA drug-label PDFs (DailyMed, public domain), expanding to
1,315 labeled examples via the `evidence-eval inject` matched-pair
operators.

**How it was generated (be precise — this matters for the paper):**

- Source corpus: `evidence-eval fetch dailymed --limit 50` (DailyMed
  v2 API, public domain SPL labels). Manifest with SHA-256 per file.
- The 263 `valid` seed examples were **AI-authored** by Claude
  (Anthropic) via parallel sub-agents, on 2026-05-16, driven by the
  `evidence-eval author` span scaffold over each PDF. Each agent
  selected facts, wrote one cited true sentence per example, and added
  two contrast-set `support_mutations` (one `contradicted`, one
  `unsupported`).
- Authoring constraints enforced per example: source spans copied
  verbatim from the PDF text extraction (adjacent line merges + CRLF
  trimming only; no paraphrase or invention of source text);
  single-span minimal citation; one true response sentence with high
  lexical overlap to the cited span; two typed mutations with varied
  contradiction mechanisms (number flip / negation / entity swap /
  polarity reversal).
- Every file passed `evidence-eval lint` (structural validity,
  wrong-class detection, duplicate-name detection) — whole-directory
  lint: 0 diagnostics.
- Faithfulness audit: a sample of files across independent shards and
  labels (amiodarone, metoprolol, dapagliflozin) was checked
  span-by-span against the source PDF text and confirmed verbatim.
  This was a spot-check, **not** an exhaustive human verification of
  all 263.

**Known limitations a reader/reviewer should weigh:**

- The seeds are AI-authored from a small set of authoring templates and
  rules. They are structurally more homogeneous than an independently
  hand-curated benchmark. Effect-size claims that depend on authoring
  diversity should be read with this in mind.
- `lint` verifies structure, not semantic faithfulness at scale. Only a
  sample was audited against source PDFs.
- Single NLI checkpoint, single retrieval config in the accompanying
  run. No human inter-annotator agreement.

**Recommended use:** treat as a strong scaffold benchmark. For a
publication, a human verification/curation pass over the seeds and
disclosure of the AI-assisted authoring method is the conservative
path.
