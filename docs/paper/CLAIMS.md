# What the paper claims

**Working title:** *Three Rules for a Citation: Decomposing Faithfulness in Retrieval-Augmented Generation*

**Author:** Bilal Sabry, Krux AI

**Status:** Draft. Revise after every prior-art read. Freeze by end of Week 4 (corpus + benchmark complete).

---

## The headline claim

In retrieval-augmented generation, "is this citation correct?" is treated as a single signal in the literature. We argue, and show on the v2 injected FDA-label benchmark (not real-world failure prevalence), that it decomposes into three independent constraints — **existence**, **in-context**, and **support** — and that under matched-pair failure injection these constraints catch **non-overlapping** classes of citation failures. Conflating them hides which lever a practitioner should pull.

## The three rules, one sentence each

| Rule | Fails when | What gate fixes it |
|---|---|---|
| **Existence** | The cited span ID does not resolve to a real span in the corpus. | A primary-key check at the validator. |
| **In-context** | The cited span exists in the corpus but was not in the retrieved set the model saw for this query. | A HashSet of allowed spans, scoped to one prompt. |
| **Support** | The span was retrieved and cited, but does not entail the claim. | An NLI cross-encoder over (span_text, sentence) pairs. |

The three are *operationally* independent: each can fail without the others failing. The empirical question is whether they are also *statistically* independent — i.e., do they catch distinct errors in practice. That is what the paper measures on the controlled benchmark.

**Scope (correctness, not causal faithfulness).** We claim citation *correctness* at the validator boundary (existence, in-context, support). We do NOT claim causal citation faithfulness — the in-context gate shows the span was *available* to the model, not that the model *relied* on it; post-rationalization is possible. Causal reliance is a separate, out-of-scope fourth dimension. All headline claims below are scoped to the v2 injected FDA-label benchmark; they do not reflect real-world failure prevalence.

## The empirical claim, more precisely

On the FDA drug-label benchmark (v2, 254 seeds → 1,270 examples via
controlled failure injection):

1. **Per-rule isolation.** Each rule catches its labeled failure class
   with F1 ≥ 0.95.
   - **Measured (`evidence-eval metrics`, DeBERTa-v3 support):**
     existence F1 = **1.000**, in-context F1 = **1.000** (target
     exceeded); support F1 = **0.929** (recall 0.992, precision 0.873 —
     the precision gap is the §5.1 NLI conservatism, not a missed
     failure). 2/3 rules exceed the target; the third is recall-strong
     and precision-bounded by the NLI checkpoint. See
     `docs/paper/rule-metrics.md`.
2. **Non-overlap.** Errors caught by rule 3 are NOT caught by rules 1
   or 2 (target: ≥80% disjoint), on the v2 injected FDA-label
   benchmark (not real-world failure prevalence).
   - **Measured: fully disjoint on the injected benchmark (100% by the
     operational blindness measure).** OutOfContext accepted by
     existence-only 254/254; Unsupported+Contradicted accepted by
     two-gate 508/508. Each failure class is invisible to the rules
     preceding its owning gate. Target far exceeded.
3. **Additive lift.** The composed validator (rules 1 + 2 + 3) improves
   should-refuse F1 over the best non-composed baseline (target:
   ≥10 points), on the v2 injected FDA-label benchmark (not real-world
   failure prevalence).
   - **Measured: +56.3 F1 points.** Ablation ladder (binary
     should-refuse classifier over 1,270 examples, DeBERTa-v3 support):
     vanilla 0.000 → existence-only 0.400 → two-gate 0.667 → three-gate
     **0.963**. Each gate roughly doubles recall (0.25 → 0.50 → 0.996).
     Baseline = best non-composed policy (existence-only, 0.400);
     in-context-alone / support-alone are not isolable in a nested
     stack. Target far exceeded. See `docs/paper/rule-metrics.md`.
4. **Natural-failure agreement.** On naturally-occurring LLM hallucinations (no failure injection), the three-rule validator catches the supplementary human-graded errors at rate ≥ Z%. [Z to be measured in Week 8.]

The contribution is **the decomposition + the empirical evidence that the decomposition holds on the controlled benchmark**, not any single component.

## Domain and scope

- **Corpus:** ~50 FDA drug labels scraped from DailyMed. Public-domain, structured, citations legally meaningful.
- **Benchmark:** 254 seeds → 1,270 examples via controlled failure injection, over verified (question, document, gold answer, gold spans) tuples spanning single-span, multi-span, negation, quantitative, and contraindication question types.
- **Failure-injection corpus:** for each example, three synthetic failure variants (existence / in-context / support), built via contrast-set methodology (Gardner et al., 2020).
- **Models:** Two open Ollama backends (Llama 3.1 8B, Qwen 2.5 7B) + one closed reference (Claude Haiku or GPT-4o-mini). NLI: `cross-encoder/nli-deberta-v3-large`.

## What the paper does NOT claim

- **Not a new architecture.** We do not propose a new ML model. The contribution is taxonomic and empirical.
- **Not generalization beyond the benchmark domain.** All claims are scoped to FDA drug labels. Reviewers should not infer generalization to legal, financial, or open-web content.
- **Not production validation.** The reference implementation (`evidence`) is open-sourced for reproducibility. We do not claim clinical or regulatory deployment readiness.
- **Not a benchmark contribution per se.** The benchmark exists to demonstrate the decomposition. It is small (254 seeds → 1,270 examples). It is not designed to supplant ALCE, HAGRID, ExpertQA, or any other existing eval.
- **Not "first to use NLI for faithfulness."** That lineage starts with Honovich '22 (TRUE) and is well-traveled. We are using NLI as one rule in a three-rule decomposition, not claiming the NLI move itself.

## Where we are likely to be attacked

These are the predicted reviewer concerns, in order of severity. Each must have a defense in the paper.

1. **"The decomposition is obvious in retrospect."** Defense: lead with the non-overlap matrix, not the taxonomy. If rules 2 and 3 catch the same errors at >50% overlap, the decomposition is window dressing. The data must force the reader to want the taxonomy.
2. **"Your injected failures are too easy."** Defense: human-eval week (Week 8) measures whether injected failures correlate with naturally-occurring failures observed in real-LLM outputs.
3. **"1,270 examples is too small."** Defense: scope all claims to the benchmark explicitly (254 seeds → 1,270 examples). Do not generalize. Reviewers respect narrow honest claims.
4. **"You didn't compare against ALCE."** Defense: ALCE subset (~50 examples) translated into our framework, run, and reported as a separate table.
5. **"Why FDA labels, not [other domain]?"** Defense: public, structured, legally meaningful, alignment with author's industry context. Future work explicitly names other domains.

Additional limitations we surface proactively rather than wait to be attacked on:

- The injected benchmark is class-balanced (failures prevalent); the +F1 / lift numbers do not reflect real-world failure prevalence, where false-refusal cost dominates. The support gate's false-refusal rate is reported explicitly rather than hidden inside an aggregate F1.
- Benchmark seeds are AI-assisted-authored and only sample-audited (full datasheet: `fda_label_bench_PROVENANCE.md`). Structural results (existence / in-context, non-overlap) are authoring-independent; support-gate numbers carry an authoring-homogeneity caveat.
- TrustNLP 2026 deadlines have passed; venue and timeline are TBD. We do not hard-code a venue claim — arXiv-first, then the next suitable ACL/EMNLP cycle.

## What success looks like

1. The non-overlap claim holds at ≥80% on the injected benchmark. (Necessary.)
2. arXiv preprint goes live by end of Week 11.
3. A venue submission lands at the next suitable cycle (venue/cycle TBD — TrustNLP 2026 deadlines passed; arXiv-first then next suitable ACL/EMNLP cycle).
4. At least one of: (a) workshop acceptance, (b) ≥500 GitHub stars on `evidence`, (c) Show HN top-page placement, (d) cited by another paper within 12 months.

If only (1) holds, the work was scientifically valid on the controlled benchmark and we publish on arXiv. If only (2)+(3) hold without acceptance, we still have a credible artifact and a real claim. The harder failure mode is (1) failing — that means the decomposition isn't empirically distinct enough and we pivot to a different angle (likely: span-level granularity + abstention-as-deployment-philosophy).

## What we don't yet know

- ~~The non-overlap percentage. Currently hand-waved as "target ≥80%."~~ **Resolved: fully disjoint on the v2 injected benchmark (100% by the operational blindness measure)** (`docs/paper/rule-metrics.md`).
- Whether DeBERTa-v3-large materially outperforms `distilbert-base-uncased-mnli` on this task. Likely yes (TRUE-paper evidence), but we'll measure.
- Whether the 30-example bootstrap pattern (98.3% NLI agreement) holds at 1,270 examples (254 seeds → 1,270). Almost certainly some regression as we hit edge cases. Plan accordingly.
- Pharma co-author for human eval. Open question; ask this week.

## Revision log

- **2026-05-15** — Initial draft. Framing synthesized from the strategic plan + the work shipped through PR #29. Reviewer reads pending.
- **2026-05-15** — First-pass prior-art reads completed by Claude (training-data knowledge, no live web). See [`prior-art-reading.md`](prior-art-reading.md) and [`related-work-table.md`](related-work-table.md). **Decomposition holds on the controlled benchmark.** One defensive caveat noted on ALCE: their construction bakes rules 1 and 2 away, which makes our work an explicit form of what ALCE smuggles into its setup. The empirical defense is to measure how often real LLMs violate those assumptions in deployment. Pending: human verification of the reads, live 2024-2025 arXiv scan, send to one trusted external reader.
- **2026-05-15** — Web-verified pass: Claude refetched all 11 papers via arXiv abstract + ar5iv HTML, corrected the first-pass entries against the actual sources, and ran the three pre-submission lock search queries. **Decomposition still holds on the controlled benchmark.** Three material corrections: GopherCite uses constrained decoding (not substring verification); Self-RAG has `IsRel` + `IsSup` (more decomposed than first-pass credited — closest existing partial decomposition); ALCE precision is redundancy-aware (stricter than first-pass credited). Three new 2026 papers surfaced and verified non-subsuming (Zhao survey, CiteGuard, FACTUM). The Self-RAG finding is the most likely reviewer-attack vector; defense paragraph drafted in `OUTLINE.md` §2. Still pending: human read of source PDFs, exact quote pulls from venue typeset, send to external reader.
