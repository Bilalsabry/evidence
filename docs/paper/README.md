# Paper workspace

Working artifacts for the in-progress paper

> **Three Rules for a Citation: Decomposing Faithfulness in Retrieval-Augmented Generation**

This directory is the paper's planning layer. The code that backs the paper's claims lives in `crates/evidence-core/`, `crates/evidence-eval/`, and the bootstrap dataset at `crates/evidence-eval/datasets/bootstrap.toml`. The paper itself ships from a separate LaTeX project once the empirical work clears.

## What's here

| File | What it is | When to update |
|---|---|---|
| [`CLAIMS.md`](CLAIMS.md) | One-page "what the paper claims." The narrowest version of the contribution we can defend. | Before Week 2; revise after every prior-art read; freeze by Week 4. |
| [`OUTLINE.md`](OUTLINE.md) | Section-by-section sketch of the workshop paper. ~8 pages, ACL short format. | Throughout. Fill in as experiments land. |
| [`prior-art-reading.md`](prior-art-reading.md) | One-paragraph notes per cited paper. The reading list and template. | Week 1, in parallel with the strategic plan's reads. |
| [`related-work-table.md`](related-work-table.md) | Matrix: each prior paper × each of the three rules, marking what they measure and what they conflate. The "we earned this paragraph" evidence. | Filled in after the reads. |
| [`preliminary-results.md`](preliminary-results.md) | Bootstrap-set run of the full harness. De-risks §5.2/§5.3: catch matrix is exactly diagonal; real-NLI failure mode is false-refusal-on-valid. Regenerates from documented commands. | Replace with full-benchmark numbers before submission. |

## Tie-points to the codebase

- The three-rule decomposition is implemented in [`evidence_core::query::ValidationPolicy`](../../crates/evidence-core/src/query/mod.rs).
- The empirical harness is [`evidence-eval`](../../crates/evidence-eval/).
- The methodology + first empirical results live in [`docs/EVAL.md`](../EVAL.md). That document is the *current* state; this directory is the *paper-shaped* synthesis of it.
- The "Closed-Loop Citation" name (used in the README and dev log) refers to the deployment philosophy — the model is bound to prompt context. Inside the paper, it's a section subtitle; the headline framing is the three-rule decomposition.

## Target venue + timeline

Primary: **TrustNLP @ ACL 2026** (workshop, deadline ~early August 2026). Concurrent **arXiv preprint** is the asset that gets cited and linked.

Skip: EMNLP 2026 main-conf short. Bar too high for a clean 3-month timeline. Defer a stronger version to ACL 2027 short.

## What to write each week

See the strategic plan (in your private notes). Short version:

- **Week 1 (this week):** Read prior art (this dir's `prior-art-reading.md`), write [`CLAIMS.md`](CLAIMS.md), send to one trusted reader.
- **Weeks 2–4:** Build the corpus + benchmark + injection set. No paper writing.
- **Weeks 5–8:** Experiments + analysis + human eval.
- **Weeks 9–12:** First draft → review → arXiv + submission.
