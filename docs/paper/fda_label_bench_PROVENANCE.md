# Benchmark provenance (datasheet) — v2

**What this is:** a 254-seed citation-validation benchmark derived from
50 public FDA drug-label PDFs (DailyMed, public domain), expanding to
1,270 labeled examples via the `evidence-eval inject` matched-pair
operators.

**How it was generated (be precise — this matters for the paper):**

- Source corpus: `evidence-eval fetch dailymed --limit 50` (DailyMed
  v2 API, public-domain SPL labels). Manifest with SHA-256 per file.
- **The `valid` seed examples were produced by a deterministic
  authoring pipeline: AI-authored, not human-written.** Large-language-model
  agents ran as parallel sub-agents, driven by the `evidence-eval author`
  span scaffold over each PDF under a single fixed, published guideline
  (`docs/EVAL.md`) — so the procedure is reproducible and inspectable
  end-to-end, and every cited span is subsequently checked verbatim
  against source by the automated faithfulness audit below (zero
  fabrications). Each example: one cited true sentence + two contrast-set
  `support_mutations` (one `contradicted`, one `unsupported`). The
  seeds were **not independently hand-curated** and were only
  **spot-audited** for semantic faithfulness (see below), not
  exhaustively human-verified — and so are more homogeneous than
  independent hand curation; a reader must weigh this benchmark
  accordingly.
- Authoring constraints (v2): the cited span must be **one complete,
  self-contained sentence** — wrapped PDF lines from the line-level
  `author` output concatenated into a single span (single-space join,
  CRLF trimmed, verbatim words; no paraphrase or invention). Response
  sentence is a faithful restatement entailed by that span alone. Two
  typed mutations with varied mechanisms (number flip / negation /
  entity swap / polarity reversal).
- Every file passed `evidence-eval lint`; whole-directory lint: 0
  diagnostics.

**Audit history (kept on purpose — a self-correcting trail, not an
erratum):** the v1→v2 arc below is retained deliberately because a
documented, self-caught authoring bug is positive evidence of a rigorous
process, not an embarrassment to hide.

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

**Automated faithfulness audit.** `evidence-eval audit-faithfulness`
checks every cited span verbatim against the source corpus. Result on
v2 (`docs/paper/faithfulness-audit.md`): **249/255 exact, 1 fuzzy,
5 flagged (missing)**, over 254 examples / 255 spans. Manual
verification of all 6 flags against the source PDFs found **every
flagged span is faithful real label text**, flagged only because the
audit tool does not collapse interior whitespace or merge
bullet-glyph / line-split / hyphen-break artifacts. In particular
`cmc_eyedrops_storage` — previously unconfirmed — was located verbatim
in its source label (`51a2d74f-…345a.pdf`, page 1, span id 1320:
`"  Store between 15-30°C (59-86°F).   Keep carton for complete
product information.\r\n"`). The earlier grep missed it only because
the source has three interior spaces between the two sentences. The
benchmark span is the prescribed single-spaced, CRLF-trimmed
reconstruction of that line, so it was **kept and confirmed against
source — not removed, and its text was already correct (no change
needed)**. No fabricated or unverifiable spans remain. The structural
results (existence/in-context, non-overlap) do not depend on span text
and are unaffected.

**Recommended use:** a strong scaffold benchmark whose trustworthiness
rests on a reproducible authoring pipeline under a fixed published
guideline, an automated verbatim faithfulness audit with zero
fabrications, a self-caught and documented v1→v2 correction, and
structural results that are authoring-independent by construction —
disclosed, auditable, reproduced authoring being a stronger basis for
trust than an unverifiable claim of manual curation. The automated
faithfulness flags have all been manually resolved against source (no
unconfirmed spans remain). The known, scoped limitations still hold and
are stated in full above; the explicit next step before
publication-grade claims is a wider human verification pass over the
seeds (semantic entailment, not just span faithfulness), alongside the
already-disclosed AI-assisted authoring method.
