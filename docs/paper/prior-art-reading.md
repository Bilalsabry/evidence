# Prior-art reading

The reading list for Week 1 of the paper timeline. ~15 hours total. Fill in each entry as you go; the notes here become the related-work section.

**The goal of these reads is to find out whether the kernel still holds.** If any one of these papers makes the three-rule decomposition claim, the paper needs to pivot. The Week 1 success criterion is a confident "no, none of them does this" — not "I've read everything."

---

## ⚠️ Verification status

Each entry has a `Status:` line. There are three levels:

1. `first-pass` — Claude-drafted from training data, no web access. Earlier version of this doc.
2. `web-verified YYYY-MM-DD via Claude+WebFetch` — Claude refetched the actual arXiv (abstract + HTML render) and corrected the first-pass against the source. **This is where every entry stands at 2026-05-15.**
3. `verified YYYY-MM-DD by <name>` — a human read the actual PDF, pulled exact quotes from the published version, and checked every claim. **No entry is at this level yet.** Critical for entries marked with 🔴 below.

The 🔴 marker flags entries where the web-verification revealed a material correction from the first-pass. For those, the human read should pay extra attention to the corrected claim.

---

## Template

Copy this block per paper:

```
## [Paper short name]

**Citation:** [Author year, title, venue]
**Read on:** YYYY-MM-DD
**Time spent:** N hours

**One-paragraph summary:**
(What does it claim? What does it measure? What's its experimental setup?)

**What it measures, in one bullet list:**
- (the metrics they actually compute)

**Where it conflates the three rules:**
- existence × in-context × support: which pairs does this paper collapse?

**The gap it leaves for us:**
- (the specific claim we can make that this paper doesn't)

**Quotable lines for the related-work section:**
> "..."

**Does this paper's claim subsume ours?**
[no / partially / yes]. If yes or partially: what's the pivot?
```

---

## Foundational (read in this order)

### Rashkin et al., 2023 — "Measuring Attribution in Natural Language Generation Models" (AIS)

**Citation:** Rashkin, H., Nikolaev, V., Lamm, M., Aroyo, L., Collins, M., Das, D., Petrov, S., Tomar, G. S., Turc, I., Reitter, D. (2023). *Measuring Attribution in Natural Language Generation Models.* TACL. (Authors verified via arXiv:2112.12870.)

**Status:** web-verified 2026-05-15 via Claude+WebFetch (arxiv abstract + ar5iv render).

**One-paragraph summary:** Introduces the **AIS** ("Attributable to Identified Sources") framework. A model output is AIS-attributable to a source iff a generic listener would conclude the output is fully derivable from the source. Operationalized as a **two-stage human-evaluation protocol**: (1) an **Interpretability Rating** that checks the output is comprehensible without seeing the source, and (2) an **AIS Rating** that checks whether all information in the output can be supported by the provided source using the "according to" test. Validated across conversational QA, summarization, and table-to-text generation.

**What it measures, in one bullet list:**
- Binary Interpretability rating per output (Stage 1).
- Binary AIS rating per (output, source) pair from human raters (Stage 2).
- Inter-annotator agreement.

**Where it conflates the three rules:**
- existence × in-context × support: **all three collapse into the AIS Rating**. The source is **assumed given by construction** — raters are shown the actual source, so existence is satisfied tautologically and the question of "was this source actually retrieved by the system" never arises. The AIS Rating is essentially our gate 3 in human-evaluation form.
- The Interpretability Rating is a *shape* check (is the sentence self-contained), not a citation check. Closest analog to our "require_citations" toggle, but applied to the answer text rather than to citation presence.

**The gap it leaves for us:**
- AIS gives one binary attribution judgment under the assumption that the source is correct. Our work makes the source-identification step itself measurable: gate 1 (is the cited span real?) and gate 2 (was it in the prompt?) are pre-conditions AIS quietly assumes.

**Quotable lines for the related-work section:**
> "[AIS evaluates] whether statements in natural language made by a system are derivable from a given underlying source." — Rashkin et al., 2023

**Does this paper's claim subsume ours?** **No.** AIS is the dominant operational definition of "attribution" we're decomposing under. Frame our paper as the explicit decomposition of what AIS bundles into one judgment.

---

### Bohnet et al., 2022 — "Attributed Question Answering: Evaluation and Modeling for Attributed Large Language Models"

**Citation:** Bohnet, B., Tran, V. Q., Verga, P., Aharoni, R., Andor, D., Baldini Soares, L., Ciaramita, M., Eisenstein, J., Ganchev, K., Herzig, J., Hui, K., Kwiatkowski, T., Ma, J., Ni, J., Sestorain Saralegui, L., Schuster, T., Cohen, W. W., Collins, M., Das, D., Metzler, D., Petrov, S., Webster, K. (2022). *Attributed Question Answering: Evaluation and Modeling for Attributed Large Language Models.* arXiv:2212.08037.

**Status:** web-verified 2026-05-15 via Claude+WebFetch.

**One-paragraph summary:** Defines the **attributed QA** task and provides a reproducible evaluation framework for it. Benchmarks a broad set of architectural approaches (retrieve-then-generate, generate-then-retrieve, generate-then-rerank) against AIS-style human evaluation. The contribution is task definition + architectural comparison, not new citation-validation methodology.

**What it measures, in one bullet list:**
- Exact-match / F1 on answer correctness.
- AIS judgment on the produced (answer, evidence) pair as the citation correctness signal.

**Where it conflates the three rules:** Inherits AIS — single binary signal bundles support, assumes existence and in-context away by construction (the evidence is held against the gold corpus and the systems studied are retrieve-then-generate, so retrieved = available).

**The gap it leaves for us:** Defines the task we operate in; doesn't measure the decomposition.

**Quotable lines for the related-work section:**
> *(first-pass: pull from the published version when verifying)*

**Does this paper's claim subsume ours?** **No.** Names the task; reuses AIS for evaluation.

---

### Gao et al., 2023 — "Enabling Large Language Models to Generate Text with Citations" (ALCE)

**Citation:** Gao, T., Yen, H., Yu, J., Chen, D. (2023). *Enabling Large Language Models to Generate Text with Citations.* EMNLP 2023.

**Status:** web-verified 2026-05-15 via Claude+WebFetch (deep read of methodology section). 🔴 Material correction from first-pass.

**One-paragraph summary:** Introduces **ALCE**, the standard benchmark for generation-with-citations. Builds three datasets (ASQA from ambiguous QA, QAMPARI from list-style QA, ELI5 from long-form QA). Defines **automatic metrics along three dimensions** — fluency, correctness, and citation quality — and validates them against human judgments. The citation-quality metrics use the **TRUE NLI model** (T5-11B fine-tuned on multiple NLI datasets) to score entailment. Reports baselines across closed and open LLMs.

**What it measures, in one bullet list:**
- **Citation recall:** for each statement, recall=1 iff it has ≥1 citation AND the concatenation of cited passages entails the statement (TRUE-NLI).
- **Citation precision:** for each citation, precision=1 iff (a) recall=1 for the parent statement, AND (b) the citation is not "irrelevant" — where irrelevant means the single passage cannot support the statement alone AND removing it doesn't change the overall support. **(This redundancy-aware definition is stricter than the first-pass entry credited.)**
- Answer correctness (exact match / claim recall) and fluency.

**Where it conflates the three rules:**
- **Existence: 🚫 assumed away.** Citations are integer indices `[1][2]` against the retrieved set. A malformed citation is a parsing artifact.
- **In-context: 🚫 assumed away.** From the paper: *"each statement s_i cites a list of passages C_i = {c_{i,1}, c_{i,2}, …}, where c_{i,j} ∈ D"* — citations are by construction in the retrieval domain. The model cannot cite outside it.
- **Support: ✅ measured well.** ALCE's citation precision is essentially our gate 3 with redundancy guards.

**The gap it leaves for us:**
- ALCE measures gate 3 carefully under the assumption that gates 1 and 2 hold. Our paper makes those assumptions measurable: when real LLMs produce citations under less constrained generation, what percent of failures are gate 1 or gate 2 (rather than gate 3)? ALCE can't ask this; we can.

**Quotable lines for the related-work section:**
> "we require LLMs to provide citations to one or a few text passages for any statement they generate" — Gao et al., 2023
> "Incorporating citations brings several benefits: (1) users can easily verify LLMs' claims with the provided citations; (2) LLMs can generate text that faithfully follows cited passages" — Gao et al., 2023

**Does this paper's claim subsume ours?** **Partially — this is the most defensive cell of the matrix.** ALCE's citation-precision metric IS our rule 3 (with redundancy guards). But ALCE bakes rules 1 and 2 into its data construction, making them invisible to the metric. Our paper positions the three rules as independent in deployment; ALCE doesn't and can't. Reviewer-facing framing: *"ALCE measures rule 3 under construction-baked assumptions; we test whether the assumptions hold in real LLM outputs."*

---

### Liu et al., 2023 — "Evaluating Verifiability in Generative Search Engines"

**Citation:** Liu, N. F., Zhang, T., Liang, P. (2023). *Evaluating Verifiability in Generative Search Engines.* Findings of EMNLP 2023.

**Status:** web-verified 2026-05-15 via Claude+WebFetch.

**One-paragraph summary:** Audits four commercial generative search engines (Bing Chat, NeevaAI, Perplexity, YouChat) on 1,500 questions across four question sources. Human raters score each (statement, cited URL) pair on whether the citation supports the statement. Headline finding: **only 51.5% of generated sentences are fully supported by their citations**, and **only 74.5% of citations actually support their associated statements**. Closest spirit-of-the-paper to ours — exposes a verifiability gap, but stops short of decomposing the failure modes.

**What it measures, in one bullet list:**
- Citation precision (human-judged): the cited URL supports the statement.
- Citation recall (human-judged): the statement's claims are supported by at least one cited URL.
- Fluency (Likert).

**Where it conflates the three rules:**
- **Existence: 🔶 noted in passing.** Liu et al. observe non-resolving URLs but don't measure them as a separate signal. Existence is *closer* to measured here than in any other paper on this list, but not as a primary metric.
- **In-context: ⬜ not applicable.** Commercial search engines don't expose a "retrieved set" — the model's training corpus blurs with the retrieval set. The "did the model see this passage" question can't be asked.
- **Support: ✅ measured.** Their precision/recall is gate 3.

**The gap it leaves for us:**
- They study deployed search engines (closed systems, no retrieval surfaced); we study RAG (open systems with a measurable retrieved set). The in-context gate can be measured in our setting and not in theirs.

**Quotable lines for the related-work section:**
> *(first-pass: pull exact quotes from the Findings of EMNLP version)*

**Does this paper's claim subsume ours?** **No.** Closest in spirit; complementary in scope.

---

## Adjacent (skim; one paragraph each)

### Menick et al., 2022 — "Teaching language models to support answers with verified quotes" (GopherCite)

**Citation:** Menick, J., Trebacz, M., Mikulik, V., Aslanides, J., Song, F., Chadwick, M., Glaese, M., Young, S., Campbell-Gillingham, L., Irving, G., McAleese, N. (2022). *Teaching language models to support answers with verified quotes.* arXiv:2203.11147 (DeepMind tech report).

**Status:** web-verified 2026-05-15 via Claude+WebFetch (ar5iv render of the mechanism section). 🔴 Material correction from first-pass.

**One-paragraph summary:** Trains a 280B-parameter LM via reinforcement learning to produce answer + verbatim quote(s) from a retrieved document. The key mechanism (corrected from the first-pass entry, which underspecified it) is **constrained decoding**: when the model is in "quote mode," the decoder is restricted so the emitted tokens *must* form a verbatim substring of the retrieved document. From the paper: *"In order to ensure the quotes are 'verbatim' with a generative approach, we introduce a special syntax for the language model to use when quoting from documents and constrain the outputs of the model to be exact quotes from the retrieved documents when in this mode."* Primary evaluation metric is **"Supported & Plausible"** (S&P) — whether the answer is plausible AND supported by the quote.

**What it measures, in one bullet list:**
- "Supported & Plausible" (S&P) score: human-rated joint plausibility + support.
- Constrained decoding guarantees quote-document fidelity *by construction*.

**Where it conflates the three rules:**
- **Existence: 🚫 enforced by construction.** Constrained decoding makes it structurally impossible for the model to emit a quote not in the document.
- **In-context: 🚫 enforced by construction.** The "retrieved document" IS the quote domain.
- **Support: 🔶 bundled with plausibility in S&P.** Their primary metric mixes plausibility (separate question) with support.

**The gap it leaves for us:**
- GopherCite is the closest prior mechanism to span-level grounding, but it's a *generative* mechanism (the model emits text, system constrains it), not a *validating* mechanism (the model emits a pointer, system checks). Critically, GopherCite cannot fail at gates 1 or 2 *by construction* — the failure modes our paper measures are eliminated by their architecture rather than measured by it.

**Quotable lines for the related-work section:**
> "we introduce a special syntax for the language model to use when quoting from documents and constrain the outputs of the model to be exact quotes from the retrieved documents when in this mode." — Menick et al., 2022

**Does this paper's claim subsume ours?** **No.** Different mechanism (generative + constrained vs. validating). Worth a paragraph in §2 explicitly because it shows the field has been groping toward gate 1+2 enforcement for years without naming it as a separable concern.

---

### Kamalloo et al., 2023 — HAGRID

**Citation:** Kamalloo, E., Jafari, A., Zhang, X., Thakur, N., Lin, J. (2023). *HAGRID: A Human-LLM Collaborative Dataset for Generative Information-Seeking with Attribution.* arXiv:2307.16883.

**Status:** web-verified 2026-05-15 via Claude+WebFetch.

**Summary + gap:** Collaborative dataset construction: GPT-3.5 drafts attributed explanations, humans rate them on informativeness and attributability. Resulting dataset supports training "models that retrieve candidate quotes and generate attributed explanations." The citation evaluation re-uses ALCE-style metrics (citation precision / recall via NLI). Same single-signal limitation. **Does not subsume.**

---

### Malaviya et al., 2024 — ExpertQA

**Citation:** Malaviya, C., Lee, S., Chen, S., Sieber, E., Yatskar, M., Roth, D. (2024). *ExpertQA: Expert-Curated Questions and Attributed Answers.* NAACL 2024.

**Status:** web-verified 2026-05-15 via Claude+WebFetch.

**Summary + gap:** 2,177 questions across 32 fields with domain-expert verification. Multidimensional evaluation that separately tracks attribution quality and factuality. Experts rate whether claims are "supported by verifiable sources" — this is essentially our gate 3 with human raters and domain expertise. Attribution is one dimension among several; it isn't itself decomposed into existence/in-context/support. **Does not subsume**, but worth flagging as the closest existing example of multi-attribute eval that we could extend with the three-rule decomposition.

---

### Gao et al., 2022 — RARR

**Citation:** Gao, L., Dai, Z., Pasupat, P., Chen, A., Chaganty, A. T., Fan, Y., Zhao, V. Y., Lao, N., Lee, H., Juan, D.-C., Guu, K. (2022). *RARR: Researching and Revising What Language Models Say, Using Language Models.* ACL 2023.

**Status:** web-verified 2026-05-15 via Claude+WebFetch.

**Summary + gap:** "Automatically finds attribution for the output of any text generation model and post-edits the output to fix unsupported content while preserving the original output as much as possible." **Revises** the model output rather than refusing it. Operates on rule 3 (support) with revision-response semantics. The deployment philosophy is the polar opposite of ours (refuse-or-resolve): RARR rewrites confidently, we refuse loudly. Worth one sentence in §2 as the contrast.

---

### Asai et al., 2024 — Self-RAG

**Citation:** Asai, A., Wu, Z., Wang, Y., Sil, A., Hajishirzi, H. (2024). *Self-RAG: Learning to Retrieve, Generate, and Critique through Self-Reflection.* ICLR 2024.

**Status:** web-verified 2026-05-15 via Claude+WebFetch (deep read of reflection-token table). 🔴 Material correction from first-pass.

**Summary + gap:** Trains an LM to emit four kinds of **reflection tokens** during generation, materially more decomposed than the first-pass entry credited:

- **`Retrieve`** {yes, no, continue} — when to invoke retrieval.
- **`IsRel`** {relevant, irrelevant} — whether the retrieved passage is relevant to the query.
- **`IsSup`** {fully supported, partially supported, no support} — whether the generated statement is supported by the passage. **Three-valued, not binary.** Closest existing analog to our gate 3.
- **`IsUse`** {1–5} — overall utility of the generation.

The decomposition is more sophisticated than I gave it credit for in the first-pass: Self-RAG separately rates **passage relevance** (`IsRel`) from **support** (`IsSup`). However, neither token measures our gate 2 (was the cited passage in the prompt set the model saw). Self-RAG's setup makes that question moot — the model only generates citations against passages it just retrieved, by construction, so there's no "in-context" failure mode to label. The reflection tokens also live *inside* the model's generation (trained via supervised fine-tuning), not as an external validator that can refuse the output.

**Where it conflates the three rules:**
- existence × in-context × support: **`IsRel` partially decomposes retrieval-relevance from `IsSup`'s support signal.** This is genuinely closer to a decomposition than most papers — but neither token addresses existence or in-context as we define them, and both are *internalized* signals trained at fine-tuning rather than *externalized* validator gates.

**Does this paper's claim subsume ours?** **No** — but it is the most likely paper a reviewer will compare us against on architectural grounds. Defense: (a) `IsRel` measures retrieval *relevance*, not citation in-context grounding; (b) the reflection tokens are inside the model (trainable signals), our gates are outside the model (externalized invariants that *refuse*). Different deployment story; different failure mode (Self-RAG: model thinks its output is fine but is wrong; evidence: model output is refused with a typed error and never reaches the user).

---

## Methodologically useful

### Es et al., 2023 — RAGAS

**Citation:** Es, S., James, J., Espinosa-Anke, L., Schockaert, S. (2023). *RAGAS: Automated Evaluation of Retrieval Augmented Generation.* arXiv:2309.15217.

**Status:** web-verified 2026-05-15 via Claude+WebFetch (deep read of metric formulas).

**Summary + gap:** Reference-free evaluation framework with **three metrics**:

- **Faithfulness:** LLM-based statement extraction + verification. Decomposes the answer into statements, scores each against the context, computes `F = |V|/|S|` where `|V|` is verified statements and `|S|` is total. Functionally our gate 3 with LLM-as-judge.
- **Answer Relevance:** Embedding-based cosine similarity between the original question and LLM-generated alternative questions derived from the answer.
- **Context Relevance:** LLM extracts question-relevant sentences from the context; `CR = relevant_sentences / total_sentences_in_context`. **This is retrieval-quality, NOT citation-grounding.** It asks whether the retrieved passages are useful for the question, not whether the model cited them appropriately.

**Where it conflates the three rules:** Faithfulness ≈ our gate 3 via LLM judge. Context Relevance evaluates retrieval, not citation gates. RAGAS therefore measures one of our three gates (support) and treats the others as someone else's problem (retrieval evaluation).

**The gap it leaves for us:** Frameworks like RAGAS adopt a single faithfulness signal because that's the dominant paradigm. We argue the right granularity is three signals: gate-decomposed validators are RAGAS-shaped tools that catch more failures.

---

### Honovich et al., 2022 — TRUE

**Citation:** Honovich, O., Aharoni, R., Herzig, J., Taitelbaum, H., Kukliansy, D., Cohen, V., Scialom, T., Szpektor, I., Hassidim, A., Matias, Y. (2022). *TRUE: Re-evaluating Factual Consistency Evaluation.* NAACL 2022.

**Status:** web-verified 2026-05-15 via Claude+WebFetch. 🔴 Minor correction: the finding is more nuanced than "NLI wins."

**Summary + gap:** Standardizes 11 factual-consistency datasets, evaluates many metrics. The actual finding (corrected from first-pass): **"large-scale NLI and question generation-and-answering-based approaches achieve strong and complementary results"** — not "NLI dominates." Recommends both NLI and QG-and-QA as starting points. Sets the NLI-for-faithfulness lineage and the *complementary-methods* framing we should adopt for our gate-3 implementation. The TRUE NLI model is what ALCE uses; if we use a DeBERTa-MNLI checkpoint, that's the same family.

**Quotable lines for the related-work section:**
> "Automatic factual consistency evaluation may help alleviate this limitation by accelerating evaluation cycles, filtering inconsistent outputs and augmenting training data." — Honovich et al., 2022

---

### Gardner et al., 2020 — Contrast Sets

**Citation:** Gardner, M., Artzi, Y., Basmova, V., Berant, J., Bogin, B., Chen, S., Dasigi, P., Dua, D., Elazar, Y., Gottumukkala, A., Gupta, N., Hajishirzi, H., Ilharco, G., Khashabi, D., Lin, K., Liu, J., Liu, N. F., Mulcaire, P., Ning, Q., Singh, S., Smith, N. A., Subramanian, S., Tsarfaty, R., Wallace, E., Zhang, A., Zhou, B. (2020). *Evaluating Models' Local Decision Boundaries via Contrast Sets.* Findings of EMNLP 2020.

**Status:** web-verified 2026-05-15 via Claude+WebFetch.

**Summary + gap:** Methodology paper. "Contrast sets involve manually perturbing test instances in small but meaningful ways that typically change the gold label." We cite this once in §4.4 as the justification for the failure-injection operators in `evidence-eval`. **Does not subsume.**

**Quotable line:**
> "Contrast sets provide a local view of a model's decision boundary, which can be used to more accurately evaluate a model's true linguistic capabilities." — Gardner et al., 2020

---

## 2024–2026 papers surfaced via web search

The three search queries from the first-pass verdict block ran on 2026-05-15. Three near-miss papers turned up that warrant an explicit "verified non-subsuming" note. None fired a pivot trigger; all three are mentioned here so the human read can confirm.

### Zhao et al., 2026 — "Attribution Techniques for Mitigating Hallucinated Information in RAG Systems: A Survey"

**Citation:** Zhao, Y., Liu, Z., Zheng, Y., Lam, K.-Y. (2026). *Attribution Techniques for Mitigating Hallucinated Information in RAG Systems: A Survey.* arXiv:2601.19927.

**Status:** web-verified 2026-05-15 (abstract only; full taxonomy not exposed on the landing page).

**Summary + verdict:** A survey paper that outlines a taxonomy of *hallucination types* in RAG (six categories per the search abstract: Unverifiability, Outdatedness, Overconfidence, Instruction Deviation, Context Inconsistency, Reasoning Deficiency) and surveys attribution-based mitigations across pre-retrieval, post-retrieval, pre-generation, and post-generation stages. **This taxonomy is over a different unit (hallucination types) than ours (citation-validation gates).** A model can be "Outdated" or "Overconfident" without ever producing a citation; our gates only fire when the model actually emits a citation. Different abstraction layer. **Does not subsume.** Position in §2: this is the survey-shaped meta-view of the field our paper measures one specific slice of.

### Choi et al., 2026 — "CiteGuard: Faithful Citation Attribution for LLMs via Retrieval-Augmented Validation"

**Citation:** Choi, Y. M., Guo, X., Fung, Y. R., Wang, Q. (2026). *CiteGuard: Faithful Citation Attribution for LLMs via Retrieval-Augmented Validation.* arXiv:2510.17853.

**Status:** web-verified 2026-05-15.

**Summary + verdict:** Reframes citation evaluation as **citation-attribution alignment** — *"whether LLM-generated citations match those a human author would include for the same text."* Evaluated on the **CiteME** benchmark (academic citation prediction). Achieves 68.1% accuracy vs. 69.2% human. **Different task.** CiteGuard is about *which academic paper would a human cite here* — a recommendation-style problem. Our work is about *did the model's citation pointer ground the claim it made in evidence the model was shown* — a validation problem. Same word ("citation"), different abstractions, different benchmarks. **Does not subsume.**

### Dassen et al., 2026 — "FACTUM: Mechanistic Detection of Citation Hallucination in Long-Form RAG"

**Citation:** Dassen, M., Kotula, R., Murray, K., Yates, A., Lawrie, D., Kayi, E., Mayfield, J., Duh, K. (2026). *FACTUM: Mechanistic Detection of Citation Hallucination in Long-Form RAG.* arXiv:2601.05866.

**Status:** web-verified 2026-05-15.

**Summary + verdict:** **Mechanistic interpretability** analysis. Treats citation hallucination as a *coordination failure between the model's attention pathway and its feed-forward pathway*, instrumented by four internal scores (Contextual Alignment, Attention Sink Usage, Parametric Force, Pathway Alignment). **This is a different layer of the stack.** FACTUM looks inside the transformer to ask *why* hallucinations occur; we look at the validator boundary to ask *which kind* of failure occurred. Complementary, not subsuming.

### One more thing the search surfaced

The systematic review *"A Systematic Review of Key Retrieval-Augmented Generation (RAG) Systems"* (arXiv:2507.18910) reports that **>1,200 RAG-related papers appeared on arXiv in 2024 alone.** Honest constraint: we cannot survey them all. The web-verification covers the ~14 most-cited / topically-closest papers; the rest is residual risk that a workshop reviewer surfaces a closer prior work we missed. Mitigation: respond honestly in rebuttal ("the cited paper appears to subsume X; we agree and have revised the framing to Y").

---

## After all reads — the verdict

Filled in 2026-05-15 after web-verified reads:

```
Verdict: KERNEL HOLDS, with one defensive caveat sharpened.

The three-rule decomposition (existence + in-context + support as
independent, separately-measured failure modes) is not made explicit in
any of the 11 papers on this list. The strongest near-misses are:

  - ALCE (Gao '23) — measures rule 3 with redundancy guards; assumes
    rules 1 and 2 away by construction. The defensive caveat from the
    first-pass STANDS and is now sharper: ALCE's redundancy-aware
    precision metric is stronger than the first-pass entry credited.
    Our work makes the ALCE-assumed-away rules visible.

  - Self-RAG (Asai '24) — separates retrieval relevance (`IsRel`) from
    support (`IsSup`). MORE decomposed than the first-pass credited,
    but: (1) IsRel is retrieval-quality, not citation-in-context;
    (2) reflection tokens are internalized model signals trained at
    fine-tuning, not externalized validator gates that refuse output.
    Different mechanism, different failure mode.

  - GopherCite (Menick '22) — closest mechanism for span-level claims,
    but via constrained decoding (model cannot emit non-verbatim
    quotes by construction). Eliminates gates 1 and 2 architecturally
    rather than measuring them.

  - Liu '23 — observes URL non-resolution in passing (closest paper to
    measuring gate 1 as a separate signal) but doesn't decompose.

  - AIS (Rashkin '23) — the dominant operational definition we
    decompose under. Single binary judgment, source assumed given.

Pivot triggers (none fired in this web-verification pass):
  1. If a missed paper explicitly names all three rules and measures
     them independently → kernel subsumed.
  2. If a missed paper measures rules 1+2+3 via one combined metric
     AND shows non-overlapping component catch → kernel partially
     subsumed; pivot to span-level granularity + refuse-or-resolve
     deployment philosophy as the headline contribution.

Material corrections from first-pass to web-verified:
  - GopherCite mechanism: constrained decoding (not substring check).
  - Self-RAG: has `IsRel` + `IsSup` (partial decomposition; not a
    single "support" signal as first-pass implied).
  - ALCE: precision includes redundancy check (stricter than I
    initially credited).
  - TRUE: actually "NLI + QG-and-QA are complementary," not "NLI wins."

Confidence: medium-high. The web-verification covers the abstract and
methodology sections of all 11 papers. Three known limits:

  1. I did not run a literature search for 2025 work that might
     subsume the kernel. The three search queries listed below are
     still owed before lock.
  2. I cannot verify quotations against the published venue typeset.
     For the workshop submission, every quote in the related-work
     section must be re-pulled from the PDF and the citation string
     verified against the venue's authoritative form.
  3. Self-RAG's partial decomposition is the most likely
     reviewer-attack vector. The defense paragraph in the OUTLINE
     §2 will need to anticipate it.

Three pre-submission lock queries — ALL THREE RUN 2026-05-15:
  1. arXiv search: "decompose attribution" OR "citation taxonomy"
     2024..2026 → surfaced Zhao '26 survey (different abstraction
     layer, does not subsume) + FACTUM (mechanistic interpretability,
     complementary).
  2. arXiv search: "in-context citation" OR "closed-set citation"
     2024..2026 → surfaced CiteGuard (academic-citation attribution,
     different task, does not subsume).
  3. ALCE follow-ups: no extension that decomposes precision into
     gate-shaped components surfaced. Tianyu Gao + Howard Yen have
     subsequent work on long-context evaluation (HELMET, LongProc)
     but not on citation-validation decomposition.

Residual risk: per the cited 2025 RAG survey, >1,200 RAG papers
landed on arXiv in 2024 alone. We've covered ~14 papers in the
verification pass; the remaining tail is a known unknown. Workshop
review will surface any closer prior art; we respond honestly if so.
```
