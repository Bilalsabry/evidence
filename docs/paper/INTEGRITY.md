# Research Integrity & Methods Transparency

This statement accompanies *Three Rules for a Citation: Decomposing
Faithfulness in Retrieval-Augmented Generation* and its artifact. It
states, in full and at equal prominence, how the benchmark was built,
how it was verified, what it does not yet establish, and why we regard a
disclosed, auditable, AI-assisted construction as a stronger integrity
posture than an unverifiable claim of manual curation. Every assertion
here is reconciled against `DATASHEET.md`,
`fda_label_bench_PROVENANCE.md`, `faithfulness-audit.md`,
`benchmark-results.md`, `CLAIMS.md`, and `ARTIFACT.md`; nothing here
extends or softens any fact disclosed there.

## Methods transparency

The `fda_label_bench` benchmark was not assembled by unverifiable manual
curation. Its 254 `valid` seeds were produced by a deterministic,
documented authoring pipeline: large-language-model sub-agents running
under a single fixed, published guideline (`docs/EVAL.md`) over a
line-level span scaffold extracted by `evidence-eval author` from 50
public-domain DailyMed FDA-label PDFs. Each seed cites exactly one
complete, self-contained sentence reconstructed verbatim from
consecutive source lines (single-space join, CRLF trimmed, no paraphrase
or invention) and carries two typed contrast-set mutations. The source
corpus is pinned by a SHA-256 manifest; every authored file passes
`evidence-eval lint` (whole-directory: 0 diagnostics). The full pipeline
— corpus fetch, injection into 1,270 matched-pair examples, all headline
metrics, and the audit — reproduces from the committed seeds with the
numbered one-command-per-step sequence in `ARTIFACT.md`. Structural
results need no model and reproduce network-free.

## Verification

An automated verbatim faithfulness audit
(`evidence-eval audit-faithfulness`) checked every cited span against the
source PDFs: over 254 examples / 255 corpus spans, 249 exact, 1 fuzzy, 5
flagged missing. All six flagged spans were manually located in their
source labels and confirmed to be faithful real label text — flagged
only because the audit tool does not collapse interior whitespace or
merge bullet-glyph, line-split, or hyphen-break extraction artifacts. No
fabricated or unverifiable span remains; no span text was changed and no
example was removed. The full audit, including the per-span manual
verification, is committed (`faithfulness-audit.md`). The structural
results (existence, in-context, non-overlap) do not depend on span text
and are unaffected by these artifacts.

## Self-correction

The pipeline is self-checking, and we report the evidence rather than
hide it. v1 (263 seeds) used a flawed guideline ("cite minimally, one
span") that, on line-wrapped PDF text, produced fragment citations. Our
own hand audit of v1's false-refused `valid` examples established this
was a benchmark-authoring bug, not an NLI finding — it had inflated the
v1 false-refusal rate by roughly 15 points. We corrected the guideline,
re-authored all seeds (v2, 254 seeds), and preserved v1 out-of-repo for
audit. The bug was found and fixed before any result was reported; the
v1→v2 arc is retained on purpose so the correction stays inspectable.

## Authoring-independence of the core result

The contribution — the three-rule decomposition and its non-overlap —
does not depend on who or what wrote the seeds. Existence and in-context
are index/string checks: exact, deterministic, model-independent, F1 =
1.000 (254/0/0 each), and the non-overlap result (OutOfContext accepted
by existence-only 254/254; Unsupported+Contradicted accepted by two-gate
508/508) is a measured operational-blindness property of the gates, not
an artifact of authoring or of the nested-policy construction. Only the
support-gate numbers carry an authoring-homogeneity caveat, and that
caveat is stated wherever those numbers appear.

## Scope and limitations (stated plainly)

These bound what the benchmark establishes; none is concealed and each
has a named next step. (1) AI-assisted seed authoring makes the seeds
more stylistically and structurally homogeneous than independent hand
curation — the primary threat a reviewer should weigh. (2) Verification
is a spot audit (3 labels, the v1 false-refusal hand audit, the full
automated faithfulness pass with manual flag resolution), not an
exhaustive human semantic pass over all 254 seeds. (3) No
inter-annotator agreement statistic exists yet. (4) Single domain (FDA
drug labels); no claim of generality. (5) The set is class-balanced and
injected — never a real-world prevalence estimate. (6) Single-span
dominance leaves composite-claim aggregation unexercised. The wider
human verification pass, the IAA statistic, and the natural-failure /
real-LLM prevalence study (§5.6) are explicitly recommended open work,
not done.

## Why this posture

Disclosed, auditable, reproduced AI-assisted construction is a stronger
integrity basis than an unverifiable assertion of manual authorship. A
manual-curation claim cannot be independently re-derived; this pipeline
can — from committed seeds, one command per step, with a verbatim audit
anyone can rerun and a self-caught bug left in the record. We adopt this
deliberately: candor about how the artifact was built, and about what it
does not yet establish, is the source of its credibility, not a
concession against it.
