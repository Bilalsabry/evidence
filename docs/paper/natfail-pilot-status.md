# Natural-failure pilot — pre-positioning status

**Date:** 2026-05-20
**Scope:** Pre-position Phase 3 of `SUBMISSION_RUNBOOK.md` (the
natural-failure study) so that when the human grader sits down, the
worksheet and inputs already exist and labeling can start immediately.

**This is NOT a §5.6 result.** No numeric outcome appears here. The
§5.6 table still requires the protocol's full sample size and
double-annotated **human** grading per
[`natural-failure-protocol.md`](natural-failure-protocol.md). This
document records what was staged and what is blocked.

---

## Environment check

| Item                              | Status                                                                 |
|-----------------------------------|------------------------------------------------------------------------|
| Ollama installed on this machine  | **Not available** (`ollama` not on PATH; `command not found: ollama`)  |
| Models pulled                     | n/a — Ollama not installed                                             |
| Held-out corpus `~/dailymed-holdout/labels/` | **Not present** on this machine                              |
| Main corpus `~/dailymed-corpus/labels/`      | Present (50 PDFs, `manifest.toml` intact)                    |
| Repo `evidence-eval natfail-gen` / `natfail-prep` subcommands | Present in source (`crates/evidence-eval/src/natfail_gen.rs`) |

Because Ollama is not installed, **no model call was attempted** —
neither dry-run nor real — per the runbook's "agent does not install
Ollama; that is the user's call" guardrail. See
[`SUBMISSION_RUNBOOK.md`](SUBMISSION_RUNBOOK.md) Phase 3 step 2 for
install instructions.

---

## What was pre-positioned

To let the user skip straight to the dry-run after installing Ollama,
the two purely-local artifacts that don't need Ollama have been
prepared.

### 1. Held-out PDF proxy (3 labels)

Since `~/dailymed-holdout/` does not exist, the documented fallback was
used: the **last 3 PDFs in the alphabetic order** of
`~/dailymed-corpus/labels/`, which are the least likely to have been
sources for v2 seed examples. From `manifest.toml`:

| UUID (filename stem)                       | Drug / product (per manifest)                                                       |
|--------------------------------------------|-------------------------------------------------------------------------------------|
| `dded6a0c-3bed-453e-a2ea-d3406f2c8976`     | AMORA HAND SANITIZER COCONUT (ALCOHOL) SPRAY — I World LLC                          |
| `e62f6f6f-59a9-4df4-91ca-a76c4de93fe0`     | EXTRA STRENGTH GAS RELIEF (SIMETHICONE) CAPSULE, LIQUID FILLED — Shield Pharma      |
| `f994dc48-413d-4b8e-8d3b-068bcb0c3a1b`     | AMPROMAX (AMPROLIUM ORAL SOLUTION) — Vetr LLC (veterinary)                          |

This is a **proxy held-out set**, not a true held-out corpus. If the
user has a real `~/dailymed-holdout/` elsewhere, they should re-run
the dry-run pointed at that directory and edit the `pdf =` lines in
`~/natfail/questions-pilot.toml` to match the new filenames.

### 2. Pilot questions file

Written to **`~/natfail/questions-pilot.toml`** (outside the repo). 5
prompts spread across the 3 labels, drawn from the categories a real
FDA-label end-user would ask about (active ingredient, warnings,
indication, dosing). Full text reproduced below for the record:

| # | PDF                                          | Prompt category    | Prompt                                                                                                                |
|---|----------------------------------------------|--------------------|-----------------------------------------------------------------------------------------------------------------------|
| 1 | `dded6a0c-...` (hand sanitizer)              | active ingredient  | What is the active ingredient in this product and at what concentration?                                              |
| 2 | `dded6a0c-...` (hand sanitizer)              | warnings           | What warnings does the label give about use of this hand sanitizer near eyes, mouth, or open wounds?                  |
| 3 | `e62f6f6f-...` (simethicone)                 | dose / frequency   | What is the recommended adult dose of this simethicone product and how often may it be taken?                         |
| 4 | `e62f6f6f-...` (simethicone)                 | indication         | For what symptoms or indication is this product intended to be used?                                                  |
| 5 | `f994dc48-...` (amprolium oral solution)     | dose / regimen     | What is the recommended dosing regimen of amprolium oral solution for the species and indication listed on this label?|

Schema matches `Question`/`QuestionsFile` in
`crates/evidence-eval/src/natfail_gen.rs` (`[[question]]` tables with
`pdf` and `prompt` string fields).

---

## What was NOT done (and why)

| Step                                  | Status   | Reason                                                                                       |
|---------------------------------------|----------|----------------------------------------------------------------------------------------------|
| Dry-run `natfail-gen --dry-run`       | Skipped  | Ollama not installed; the runbook reserves install for the user. Dry-run itself doesn't call Ollama but the natural next step (real run) does, and we want the user to verify install + dry-run together. |
| Real `natfail-gen` with `--limit 5`   | Skipped  | No Ollama, no models pulled.                                                                  |
| `natfail-prep` worksheet generation   | Skipped  | No `model_answers_pilot.toml` to feed it.                                                     |
| Human grading                         | Not started | This is the user's job by protocol — the agent is forbidden from grading.                  |
| §5.6 headline result                  | Untouched | Out of scope: pilot N=5 is for harness-validation, not the publishable table.                |

**No worksheet path exists yet.** The intended path once the user
proceeds is `~/natfail/worksheet-pilot.md` and the model-answers TOML
at `~/natfail/model_answers_pilot.toml`.

---

## Resume instructions for the user

After installing Ollama and pulling at least one supported model
(`llama3.1:8b` or `qwen2.5:7b`):

```sh
# 1. Verify install + models
ollama list

# 2. Confirm the pre-staged questions still look right; edit if you
#    have a real held-out corpus to point at instead.
$EDITOR ~/natfail/questions-pilot.toml

# 3. Dry-run against whichever PDF dir applies
evidence-eval natfail-gen \
  --pdf-dir ~/dailymed-corpus/labels \
  --questions ~/natfail/questions-pilot.toml \
  --models llama3.1:8b \
  --dry-run \
  --output ~/natfail/plan.toml

# 4. Real run (small)
evidence-eval natfail-gen \
  --pdf-dir ~/dailymed-corpus/labels \
  --questions ~/natfail/questions-pilot.toml \
  --models llama3.1:8b \
  --limit 5 \
  --output ~/natfail/model_answers_pilot.toml

# 5. Build the human-annotation worksheet
evidence-eval natfail-prep ~/natfail/model_answers_pilot.toml \
  --output ~/natfail/worksheet-pilot.md

# 6. Open the worksheet and fill in the human-label column
$EDITOR ~/natfail/worksheet-pilot.md
```

The worksheet's "human label" column **stays blank** until the user
fills it; the harness will not deserialize the dataset until each
example carries a real `class` value
(see `docs/paper/natural-failure-harness.md`, "What it does NOT do").

---

## One-paragraph caveat (carry this forward)

This pilot is harness-validation only: N=5 across 3 PDFs, single
model, single annotator (user), proxy held-out set instead of a true
held-out corpus. It establishes that the `natfail-gen` / `natfail-prep`
loop works end-to-end on this machine and gives the user a worksheet
to start labeling against. The publishable §5.6 natural-failure result
still requires the full sample size, the real held-out corpus, the
double-annotation protocol, and human grading per
[`natural-failure-protocol.md`](natural-failure-protocol.md). No
numeric outcome from this pilot may be quoted as a §5.6 result.
