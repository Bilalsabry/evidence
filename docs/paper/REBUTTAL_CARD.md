# Rebuttal battle card

> Read-aloud rebuttal sheet. Bullet form, ≤2 pages, every number traces
> to a committed doc. Voice: tight, first-person plural.

**Thesis (single line).** Citation correctness decomposes into three
operationally-independent gates; measured on a controlled FDA-label
benchmark, the structural rules are exact and the rules are 100%
non-overlapping.

---

## The 8 attacks (in priority order)

### 1. *"100% non-overlap is baked in by the nested policies."* (critical)
- **Reply.** It is a *measured blindness property*, not policy nesting. Out-of-context spans are real corpus text, so the existence gate sees "present" no matter where it sits — measured OOC accepted by existence-only **254/254 (100%)**. Support failures perturb the same in-prompt span, so both structural gates pass by construction of the failure — measured unsupported+contradicted accepted by two-gate **508/508 (100%)**. The disjointness would hold under independent evaluation.
- *Evidence.* `rule-metrics.md` §5.3; `section5-results.draft.md` §5.3 ("the kernel").

### 2. *"It's an AI-authored benchmark — you can't trust the seeds."* (critical)
- **Reply.** Disclosed and audited. Automated faithfulness audit over 254 examples / **255 spans: 249 exact / 1 fuzzy / 5 flagged**, every flag manually located in source PDFs and confirmed verbatim — **zero fabrication**. A v1→v2 authoring bug was self-caught, documented, and fixed (49% false-refusal decomposed into ~15 pts bug + ~29% genuine NLI conservatism). Structural results (F1 = 1.000, non-overlap) are authoring-independent — they are index/string checks on span text we did not write.
- *Evidence.* `faithfulness-audit.md` totals + manual verification; `fda_label_bench_PROVENANCE.md`; `DATASHEET.md`.

### 3. *"The decomposition is obvious in retrospect."* (high)
- **Reply.** Obvious ≠ measured. Before the experiment it was a live possibility that the support gate would also catch out-of-context errors, collapsing rules 2 and 3 into one signal — the "window dressing" failure mode named pre-registration. The data forces the three-way split: OOC 254/254 invisible to existence-only, support 508/508 invisible to two-gate. We lead with the non-overlap matrix, not the taxonomy.
- *Evidence.* `CLAIMS.md` ("window dressing" pre-reg); `rule-metrics.md` §5.3.

### 4. *"Self-RAG / GopherCite already do this."* (high)
- **Reply.** Neither externalizes the three gates nor measures non-overlap. Self-RAG internalizes `IsRel` / `IsSup` as reflection tokens — there is no "was this span in the prompt for *this* query" gate, and no validator that refuses output. GopherCite enforces existence/in-context by constrained decoding, so the failures never occur and non-overlap is unmeasurable. We credit both as the closest partial precedents.
- *Evidence.* `paper-draft.md` §2 ("The gap"); `related-work-table.md`.

### 5. *"This is just ALCE."* (high)
- **Reply.** ALCE/HAGRID/ExpertQA restrict citations to integer markers into the retrieved set, making existence and in-context true *by construction* — therefore unmeasurable. We measure exactly the two dimensions ALCE assumes away. We do not claim to supplant ALCE; an ALCE-subset translation is named as future work and marked `[OPEN — not run]`.
- *Evidence.* `paper-draft.md` §2 "RAG citation benchmarks"; `CLAIMS.md`.

### 6. *"Support F1 is below your 0.95 target — it over-refuses valid claims."* (high)
- **Reply.** Recall is **0.992** (504/508 caught; tp/fp/fn = 504/73/4) — the gate does not miss bad citations. The precision gap (**0.873**, F1 **0.929 [0.91, 0.94]** 95% bootstrap CI) is the separately-argued §5.1 finding: NLI-as-support-gate is conservative even with DeBERTa-v3 MNLI. For pharmaceutical text this is the correct default — **refuse-or-resolve**, not silent acceptance. The decomposition's value does not rest on this number.
- *Evidence.* `rule-metrics.md` §5.2; `paper-draft.md` §3.5.

### 7. *"Your injected failures are too easy."* (high)
- **Reply.** The in-context operator is matched-pair: the span's exact text is duplicated to a fresh ID *outside* the prompt and the citation repointed — only in-context status varies, topicality is held fixed (contrast-set methodology). Structural results (F1 = 1.000) do not depend on failure subtlety. The natural-failure run is the intended realism check and is honestly marked `[OPEN — not run]`.
- *Evidence.* `paper-draft.md` §4.4 (matched-pair); `natural-failure-protocol.md`.

### 8. *"No latency / cost numbers — it's a paper claim, not an engineering one."* (medium)
- **Reply.** Now measured. Harness-level wall-clock over 1,270 examples / 5,080 timed calls: structural gates **140.5 µs / call (2.5%)**, support (NLI) **5.6 ms / call (97.5%)** of a 5.7 ms total. The two exact structural gates are ~40× cheaper than the NLI pass — run them unconditionally, apply the expensive semantic gate only where they pass. Reported honestly as harness-level (mock-baseline vs real-NLI delta), order-of-magnitude.
- *Evidence.* `cost-latency.md` §5.7.

---

## Things we will NOT defend (we agreed)

- **The natural-failure run (§5.6) is not done.** Real LLM hallucinations graded into the taxonomy are the right realism check; we have a protocol, not a result. We mark it `[OPEN — not run]` everywhere, never imply otherwise, and never present injected numbers as deployment numbers. *Pointer:* `natural-failure-protocol.md`; `CLAIMS.md` claim 4.
- **AI-authoring homogeneity on the support gate.** Structural results are authoring-independent, but support-gate precision (0.873) carries a genuine homogeneity caveat: seeds are AI-authored under a single corrected guideline, with span faithfulness fully verified but only spot-audited entailment (3 labels) on top of the v1 false-refusal audit. A wider human entailment pass is the conservative path and has not been done. *Pointer:* `DATASHEET.md`; `fda_label_bench_PROVENANCE.md` "Known limitations".

---

## Minimal scope statement (quote verbatim if pushed)

> We claim a measured three-way decomposition of citation correctness on
> a controlled, disclosed FDA-label benchmark — structural rules exact,
> rules 100% non-overlapping, +56.3 F1 over the strongest non-composed
> baseline — and we claim nothing about real-world failure prevalence,
> causal faithfulness, or cross-domain generalization.
