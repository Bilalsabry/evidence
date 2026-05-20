# Natural-failure generation harness

`evidence-eval natfail-gen` removes the last manual step in Phase 3 of
[`SUBMISSION_RUNBOOK.md`](SUBMISSION_RUNBOOK.md): converting raw local
Ollama outputs into the natfail TOML schema. It does NOT change the §5.6
result. The headline natural-failure table still requires human
double-annotation per [`natural-failure-protocol.md`](natural-failure-protocol.md).

## What it does

For each `(model, PDF, question)` tuple it: extracts the PDF spans (via
`evidence_core::ingest::pdf::extract`, numbered globally from 1), presents
the numbered spans + the question to a locally-running Ollama model, asks
for a single JSON object `{"sentence": "...", "cited_spans": [<ids>]}`,
parses the result tolerantly (handles JSON embedded in prose), and emits
one `[[example]]` block per success in the standard dataset schema.

## What it does NOT do

- It does not assign a `class`. Every emitted example carries a
  `# class = "FILL_IN_HUMAN_LABEL"` comment placeholder. The file will
  not deserialize until a human fills the class — which is the point.
- It does not grade, score, or auto-label any output. Validator-side
  verdicts come from `natfail-prep`, after humans label the class.
- It does not invent or fall back to mock data. Failures (Ollama down,
  model not pulled, JSON unparseable) are surfaced as skips and a clean
  exit code.

## Usage

```sh
evidence-eval natfail-gen \
  --pdf-dir ~/dailymed-holdout/labels \
  --questions ~/natfail/questions.toml \
  --models llama3.1:8b,qwen2.5:7b \
  --output ~/natfail/model_answers.toml
```

`--dry-run` prints the planned tuples without calling Ollama.
`--limit N` caps the cartesian product for development. Exit code 4 means
Ollama is unreachable; 5 means every tuple skipped. Pipe the output to
`evidence-eval natfail-prep` once humans have filled the class column.
