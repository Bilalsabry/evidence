# Preliminary results (bootstrap set)

> **Status:** preliminary, run on the 30-example **bootstrap** dataset
> (not the 300-example FDA-label benchmark). These numbers exist to
> validate the empirical *kernel* end-to-end and to de-risk §5 before
> the full benchmark is authored. Every table here regenerates from the
> commands below. Replace with full-benchmark numbers before submission.

Generated: 2026-05-16. Harness version: `evidence-eval` @ commit on
`main` after PR #41. NLI model: `Xenova/distilbert-base-uncased-mnli`
(the v0.3 default; the paper will move to DeBERTa-v3-large per §5.1).

---

## Reproduce

```sh
# 1. Augment the 30 valid+failure seeds with deterministic injection.
evidence-eval inject crates/evidence-eval/datasets/bootstrap.toml \
    --output /tmp/injected.toml          # → 44 examples

# 2. Deterministic structural run (mock support checker).
evidence-eval run /tmp/injected.toml --output report_mock.md

# 3. Real NLI cross-encoder for the support gate.
evidence-eval run /tmp/injected.toml --real-nli --output report_nli.md
```

`inject` expands the 5 `valid` seeds into 10 structural variants
(existence + in-context) plus 4 author-supplied support mutations, for
**58 examples** total (44 written + the matrix runs 4 policies × the
non-`valid`-filtered set = 176 (class, policy) rows).

---

## Result 1 — the rules catch disjoint classes (the kernel)

Mock-support run. Each cell is `accepted / refused` for the
(class, policy) pair. **176 / 176 rows match the expected matrix** —
the validator implements the decomposition exactly.

| class \ policy | vanilla_rag | existence_only | two_gate | three_gate |
|---|---|---|---|---|
| Valid           | 5 / 0  | 5 / 0  | 5 / 0  | 5 / 0  |
| Uncited         | 5 / 0  | 0 / 5  | 0 / 5  | 0 / 5  |
| FabricatedSpan  | 10 / 0 | 0 / 10 | 0 / 10 | 0 / 10 |
| OutOfContext    | 10 / 0 | 10 / 0 | 0 / 10 | 0 / 10 |
| Unsupported     | 7 / 0  | 7 / 0  | 7 / 0  | 0 / 7  |
| Contradicted    | 7 / 0  | 7 / 0  | 7 / 0  | 0 / 7  |

**Read the columns as a cumulative ablation.** Each failure class is
first refused by exactly one *marginal* gate, and every earlier gate
*accepts* it:

| Failure class | First caught by | Earlier gates accept it? |
|---|---|---|
| Uncited / FabricatedSpan | existence (gate 1) | vanilla: yes |
| OutOfContext | in-context (gate 2) | vanilla **and** existence: yes (10/0) |
| Unsupported / Contradicted | support (gate 3) | vanilla, existence **and** two-gate: yes (7/0) |

The `OutOfContext` row is the load-bearing one: existence-only accepts
all 10 (the cited span *does* exist), and only the in-context gate
refuses them. Likewise the support classes survive every gate until
gate 3. No class is caught by two different marginal gates → the gates
partition the failure space on this bootstrap set. This is the §5.3
non-overlap claim, demonstrated structurally (bootstrap set, not the
FDA-label benchmark; not real-world failure prevalence).

---

## Result 2 — real-NLI support gate (the separately-measured column)

Same set, real cross-encoder swapped in for the support gate.
**173 / 176 rows match** — 3 disagreements, all isolated to the
support gate:

| class \ policy | vanilla_rag | existence_only | two_gate | three_gate |
|---|---|---|---|---|
| Valid           | 5 / 0  | 5 / 0  | 5 / 0  | **3 / 2** |
| Uncited         | 5 / 0  | 0 / 5  | 0 / 5  | 0 / 5  |
| FabricatedSpan  | 10 / 0 | 0 / 10 | 0 / 10 | 0 / 10 |
| OutOfContext    | 10 / 0 | 10 / 0 | 0 / 10 | 0 / 10 |
| Unsupported     | 7 / 0  | 7 / 0  | 7 / 0  | 0 / 7  |
| Contradicted    | 7 / 0  | 7 / 0  | 7 / 0  | 0 / 7  |

The three disagreeing rows:

1. `valid_two_spans_one_sentence` — **false refusal** (NLI judged a
   genuinely-supported sentence `unsupported`).
2. `valid_chunk_with_span_range` — **false refusal**, same cause.
3. `valid_chunk_with_span_range__inject_support_contradicted_0` —
   correctly **refused**, but labeled `unsupported` instead of
   `contradicted`. The failure is *caught*; only the 3-way reason is
   misattributed.

What this says about the support gate at this (tiny) sample:

- **Detection recall = 100%.** Every injected `unsupported` (7/7) and
  `contradicted` (7/7) is refused. Zero misses.
- **Valid retention = 3/5.** Two of five valid sentences are
  false-refused. With "refuse or resolve" deployment this is a
  precision cost, not a safety failure, but it is the real weakness and
  it lands exactly where the paper predicts (NLI error propagates;
  measured as its own column, not hidden in an aggregate).
- **Reason fidelity** within the support gate is imperfect
  (1 unsupported↔contradicted swap). Detection ≠ 3-way labeling; the
  paper should report them separately.

The distilbert-MNLI default is weak; moving to DeBERTa-v3-large (§5.1)
should lift valid-retention. That swap is the obvious first experiment
on the full benchmark.

---

## Result 3 — first real DailyMed label (end-to-end)

First hand-authored example off the fetched corpus: a single-span
quantitative claim from an OTC sunscreen label (Hampton Sun SPF30,
active-ingredient concentration). One `valid` seed + 2 author-supplied
support mutations → `inject` → `run --real-nli`. **19 / 20 rows match.**

| class \ policy | vanilla | existence | two-gate | three-gate |
|---|---|---|---|---|
| Valid          | 1 / 0 | 1 / 0 | 1 / 0 | **0 / 1** |
| FabricatedSpan  | 1 / 0 | 0 / 1 | 0 / 1 | 0 / 1 |
| OutOfContext    | 1 / 0 | 1 / 0 | 0 / 1 | 0 / 1 |
| Unsupported     | 1 / 0 | 1 / 0 | 1 / 0 | 0 / 1 |
| Contradicted    | 1 / 0 | 1 / 0 | 1 / 0 | 0 / 1 |

**Every injected failure is caught at exactly its gate on real label
text** (16/16 injected rows) — the structural kernel survives contact
with a real PDF, not just the synthetic bootstrap. The single miss is
again the NLI false-refusal-on-valid, and this example pins the
*mechanism* precisely:

- The claim — *"the active ingredient … is zinc oxide at a
  concentration of 20%"* — is trivially true; the label literally reads
  `Zinc Oxide 20%` under `Active Ingredients`.
- Cited to the single span `"Zinc Oxide 20%"`, `distilbert-MNLI` rates
  the pair *neutral* — too weak to bridge two words to the elaborated
  sentence.
- Adding the `"Active Ingredients"` header span to the citation **did
  not help and cannot**: `NliSupportChecker` is per-span strict-wins
  (`nli.rs`) — it checks each cited span independently and any single
  `Neutral` sinks the whole answer. A header span is neutral w.r.t. the
  concentration claim, so a richer citation is structurally guaranteed
  to refuse here.

So the false-refusal is the product of *two* compounding factors, both
real and both paper-relevant: a weak NLI model **and** conservative
strict-wins aggregation. It is not an authoring artifact — the example
is genuinely valid. This is a clear motivation for the
§5.1 DeBERTa-v3-large upgrade, and it surfaces a §3.4 design question:
per-span strict-wins vs. concatenated-evidence aggregation. We keep
strict-wins (conservative; "every citation must hold on its own" is a
stricter safety claim) and note concatenated-evidence as future work.

Reproduce: see [`docs/EVAL.md`](../EVAL.md) "Authoring the benchmark".

---

## Caveats (do not over-read these)

- **Bootstrap, not the benchmark.** 30 hand seeds → 58 augmented.
  n is far too small for confidence intervals. Result 1 is structural
  (it would hold at any n by construction of the validator); Result 2's
  rates (3/5, 100%) are illustrative, not measurements.
- Mock-mode 100% is *by construction* — it tests validator wiring, not
  model behavior. It is evidence the harness is correct, not a result.
- Injection is synthetic. The §4.5 natural-failure supplement still
  matters and is not addressed here.
- Single NLI model, single retrieval config, no latency numbers yet.

## What this de-risks for the paper

- §5.2 (per-rule isolation) and §5.3 (non-overlap): the mechanism works
  end-to-end and the catch matrix is exactly diagonal. The kernel is
  not at risk; only its *scale* and *effect size* are open.
- §5.1/§5.6: the NLI gate's failure mode is a false-refusal-on-valid
  pattern, not a missed-failure pattern — good for a safety framing,
  and a concrete motivation for the DeBERTa upgrade.
- §3.4: Result 3 surfaces a real aggregation design choice (per-span
  strict-wins vs. concatenated-evidence). Decided: keep strict-wins,
  document the alternative. Better raised in the paper than discovered
  by a reviewer.
- The kernel now holds on a real DailyMed PDF (Result 3), not only the
  synthetic fixture — the §4.1 corpus path and the §4.4 injection
  operators work on production label text.
