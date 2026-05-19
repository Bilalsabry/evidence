# Submission Runbook — what *you* do, step by step

Everything that can be automated has been. What remains needs human
judgment or real human-labeled data. This is the exact, ordered,
copy-pasteable path from here to a credible submission. Follow top to
bottom. Each step has: **what**, **why**, **do this**, **done when**.

- Repo (local): `/Users/bilalsabry/code/evidence`
- Repo (GitHub): https://github.com/Bilalsabry/evidence
- Open a terminal and run this once per session:
  ```sh
  cd ~/code/evidence && source "$HOME/.cargo/env"
  ```
- The verified NLI model id used everywhere below:
  `lquint/DeBERTa-v3-base-mnli-fever-anli-onnx`

**Realistic timeline:** ~1.5–2.5 focused weeks. The two big human items
(Phase 2 audit, Phase 3 natural-failure) are the credibility levers —
do not skip them for a rushed deadline. TrustNLP 2026 is **closed**
(deadlines 5 Mar / 10 Apr 2026). Plan = arXiv-first, then the next ACL
Rolling Review / workshop cycle. Do **not** rush EMNLP ARR (25 May) —
6 days is not enough to do Phases 2–3 properly, and a rushed AI-assisted
submission is exactly what gets desk-rejected.

---

## Phase 0 — Orient (30–45 min, read-only)

**Why:** you must be able to defend every sentence; first understand
the whole argument and where each number comes from.

**Do this — read in this order** (all in `docs/paper/`):

1. [`PAPER_EXPLAINER.md`](PAPER_EXPLAINER.md) — the whole idea end to end.
2. [`paper-draft.md`](paper-draft.md) — the actual §1–§7 draft.
3. [`CLAIMS.md`](CLAIMS.md) — the precise claims + measured actuals.
4. [`rule-metrics.md`](rule-metrics.md) — §5.2/§5.3/claim-3 numbers (with CIs).
5. [`nli-comparison.md`](nli-comparison.md) — §5.1 distilbert vs DeBERTa.
6. [`cost-latency.md`](cost-latency.md) — §5.7 measured cost split.
7. [`benchmark-results.md`](benchmark-results.md) — v1→v2 audit story.
8. [`faithfulness-audit.md`](faithfulness-audit.md) — zero-fabrication audit.
9. [`DATASHEET.md`](DATASHEET.md) + [`fda_label_bench_PROVENANCE.md`](fda_label_bench_PROVENANCE.md) — benchmark disclosure.
10. [`related-work.md`](related-work.md) — §2 + reference list.
11. [`reviewer-responses.md`](reviewer-responses.md) — every attack + your answer.
12. [`INTEGRITY.md`](INTEGRITY.md) — the methods-transparency posture.

**Done when:** you can state, from memory, the three rules, the
headline numbers, and the one honest weakness (support-gate
conservatism + AI-assisted benchmark, both disclosed).

---

## Phase 1 — Own the prose (4–8 hrs, required)

**Why:** the draft is solid and scoped but it is not yet *your*
sentences. Reviewers can smell un-owned text. Every number must also be
re-checked against its source table.

**Do this:**

1. Open `docs/paper/paper-draft.md`. Read it sentence by sentence.
   Rewrite anything that isn't how *you* would say it. Keep the Scope
   section, §3.0 definitions, §6 limitations, and the `[OPEN]` markers
   intact (those are load-bearing).
2. Cross-check every number in the draft against its source doc:
   - F1 / CI / non-overlap / lift → `rule-metrics.md`
   - distilbert vs DeBERTa → `nli-comparison.md`
   - cost split → `cost-latency.md`
   - audit counts → `faithfulness-audit.md`
   If any number disagrees, the source doc wins — fix the draft.
3. Read `related-work.md` and confirm you can defend the gap paragraph.
   Spot-check 3 citations against the real arXiv pages (links are in
   that file's reference list).
4. Commit your edits:
   ```sh
   cd ~/code/evidence && git add docs/paper && \
   git commit -m "paper: author revision pass" && git push
   ```

**Done when:** you have read every sentence, every number traces to a
source doc, and you would be comfortable defending each in a rebuttal.

---

## Phase 2 — Human audit of the benchmark (1–2 days, the #1 credibility lever)

**Why:** the benchmark seeds are AI-generated (disclosed). The
automated audit found zero fabrications, but reviewers will still ask
"did a human check these?" A documented human pass converts your
biggest soft spot into a strength.

**Do this:**

1. Re-run the automated audit to get the current flag list:
   ```sh
   evidence-eval audit-faithfulness \
     --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
     --corpus ~/dailymed-corpus
   ```
   Expected: `249 exact, 1 fuzzy, 5 flagged` — all already verified
   benign in `faithfulness-audit.md`.
2. Decide scope. **Minimum defensible:** a stratified random sample of
   **50** of the 254 seeds (mix of fact types). **Stronger:** all 254.
   Pull a sample list:
   ```sh
   grep -n 'name = ' crates/evidence-eval/datasets/fda_label_bench.toml \
     | shuf -n 50 > ~/audit-sample.txt
   ```
3. For each sampled example, open the dataset file, find the block, and
   check three things against the source PDF:
   - **span faithful?** Run `evidence-eval author --pdf ~/dailymed-corpus/labels/<file>.pdf`
     and confirm the cited span text is real label text.
   - **answer entailed?** Does the cited span, read alone, support the
     response sentence?
   - **mutations correct?** Is the `contradicted` one actually
     contradicted and the `unsupported` one actually off-topic?
4. Record each as ✓ / fix / drop in a simple sheet. If a second person
   can rate even 30 of them, compute Cohen's κ (any online κ calculator)
   — that single number kills the "no inter-annotator agreement" attack.
5. Apply fixes to `crates/evidence-eval/datasets/fda_label_bench.toml`,
   then **re-validate**:
   ```sh
   evidence-eval lint crates/evidence-eval/datasets/fda_label_bench.toml
   evidence-eval audit-faithfulness \
     --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
     --corpus ~/dailymed-corpus
   ```
6. Write the result (sample size, # fixed, # dropped, κ) into
   `docs/paper/DATASHEET.md` under Limitations, and into
   `docs/paper/fda_label_bench_PROVENANCE.md`. Commit.
7. If you fixed/dropped examples, regenerate the numbers (Phase 4).

**Done when:** DATASHEET records a real human-audited sample (N, κ,
fixes), and lint + audit are clean.

---

## Phase 3 — Natural-failure study (2–4 days, the #2 lever, claim 4 / §5.6)

**Why:** the benchmark uses *injected* failures. The standard reviewer
attack is "injected failures are too easy." The defense is real LLM
outputs (no injection), human-graded. The protocol is pre-written:
[`natural-failure-protocol.md`](natural-failure-protocol.md). Read it
first.

**Do this:**

1. Pick ~15 FDA labels held out from the 50 in `~/dailymed-corpus`
   (fetch more if needed: `evidence-eval fetch dailymed --output
   ~/dailymed-holdout --limit 20`). Write ~20 factual questions across
   them.
2. Install a local model runner (free, offline):
   - Ollama: https://ollama.com/download — then:
     ```sh
     ollama pull llama3.1:8b && ollama pull qwen2.5:7b
     ```
3. For each (question, label) ask each model to answer **with citations
   to span ids** (the system already ships an Ollama backend; the
   prompt format is in `crates/evidence-cli`). Capture each answer as
   an `[[example]]` in the dataset TOML schema with the model's actual
   `cited_spans`, `class` left blank for the human label.
   - **This conversion step is the only fiddly part.** It is the one
     remaining thing I can build for you on request: a small harness
     that calls Ollama and emits the natfail TOML. Ask and it's a
     same-day add.
4. Generate the annotation worksheet:
   ```sh
   evidence-eval natfail-prep ~/natfail/model_answers.toml \
     --real-nli --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
     --output ~/natfail/worksheet.md
   ```
   This gives every item the validator's verdict + a blank human-label
   column.
5. You + one other person independently label each item into the
   taxonomy (valid / uncited / fabricated_span / out_of_context /
   unsupported / contradicted / partial / ambiguous). Compute Cohen's κ.
6. Fill the §5.6 table (Human count vs validator-caught, precision,
   recall per class) into `docs/paper/section5-results.draft.md` §5.6,
   replacing the `[OPEN]` marker. Commit.

**Done when:** §5.6 has a real, human-graded table with κ reported, and
the `[OPEN]` marker is gone.

---

## Phase 4 — Finalize & reproduce all numbers (30 min, required)

**Why:** the committed paper numbers must exactly match a clean
reproduction. Full guide: [`ARTIFACT.md`](ARTIFACT.md).

**Do this:**
```sh
cd ~/code/evidence && source "$HOME/.cargo/env"
cargo install --path crates/evidence-eval --force
evidence-eval inject crates/evidence-eval/datasets/fda_label_bench.toml \
  --output /tmp/inj.toml
evidence-eval metrics /tmp/inj.toml --real-nli \
  --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
  --output docs/paper/rule-metrics.md
evidence-eval compare /tmp/inj.toml \
  --candidate-nli lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
  --output docs/paper/nli-comparison.md
evidence-eval latency --dataset /tmp/inj.toml --real-nli \
  --nli-model lquint/DeBERTa-v3-base-mnli-fever-anli-onnx \
  --output docs/paper/cost-latency.md
evidence-eval audit-faithfulness \
  --dataset crates/evidence-eval/datasets/fda_label_bench.toml \
  --corpus ~/dailymed-corpus --output docs/paper/faithfulness-audit.md
git add docs/paper && git commit -m "paper: regenerate final numbers" && git push
```
Then re-check the draft's numbers against the regenerated docs
(Phase 1 step 2).

**Done when:** every paper number was regenerated from the committed
dataset and the draft matches.

---

## Phase 5 — Format & submit (1 day)

**Why:** venues require their template, anonymization, and an artifact
appendix.

**Do this:**

1. **Pick the target.** TrustNLP 2026 is closed
   (https://trustnlpworkshop.github.io/). Realistic path:
   - **arXiv first** (no deadline, establishes priority):
     https://arxiv.org/submit — category `cs.CL`.
   - Then the next **ACL Rolling Review** cycle:
     https://aclrollingreview.org/dates (submit anytime; commit to a
     venue later). EMNLP 2026: https://2026.emnlp.org/ .
2. **Get the template.** ACL LaTeX style files:
   https://github.com/acl-org/acl-style-files . Port `paper-draft.md`
   + `related-work.md` into the template. Keep it within the workshop
   page limit (typically 4–8 pages + unlimited references).
3. **Anonymize** for ARR/workshop double-blind: remove your name,
   "Krux", the GitHub URL, and any identifying phrasing from the
   submitted PDF (keep them in the camera-ready / arXiv version).
4. **Artifact appendix.** Use [`ARTIFACT.md`](ARTIFACT.md) verbatim as
   the reproducibility appendix; tag the repo at the submission commit:
   ```sh
   cd ~/code/evidence && git tag paper-v1 && git push --tags
   ```
5. **Disclosure.** Keep the AI-assisted-authoring statement
   (`DATASHEET.md` / `INTEGRITY.md`) — most venues now require an AI-use
   statement; you already have a strong one. Do not remove it.
6. Submit arXiv first; then ARR.

**Done when:** PDF compiled in the venue template, anonymized,
artifact-tagged, AI-use statement included, submitted.

---

## Phase 6 — Pre-submission gate (do not skip)

Tick every box before you hit submit:

- [ ] Phase 1: every sentence read & owned; every number traces to a source doc.
- [ ] Phase 2: human audit done, N + κ + fixes recorded in DATASHEET; `lint` + `audit-faithfulness` clean.
- [ ] Phase 3: §5.6 has a real human-graded table + κ; no `[OPEN]` left in §5.6.
- [ ] Phase 4: all numbers regenerated from the committed dataset; draft matches.
- [ ] No `[verify]` / `[unverified]` / placeholder anywhere in `related-work.md` or the draft.
- [ ] Every citation checked against its real arXiv/venue page.
- [ ] Scope section (correctness ≠ causal faithfulness) present.
- [ ] AI-use / benchmark-disclosure statement present and accurate.
- [ ] Template-formatted, within page limit, anonymized.
- [ ] `ARTIFACT.md` is accurate; repo tagged `paper-v1`.
- [ ] One trusted external reader has done a "where does this overclaim" pass on §5 and the gap paragraph.

When all boxed: arXiv → ARR. Done.

---

## The one thing I can still build to make Phase 3 trivial

The only fiddly manual step in this whole runbook is converting raw
Ollama outputs into the natfail TOML (Phase 3, step 3). Say the word
and I'll add a generation harness that calls your local Ollama models
over the question set and emits `model_answers.toml` directly — that
removes ~80% of Phase 3's manual effort. Everything else here is
genuinely yours: judgment, human labeling, and your voice.
