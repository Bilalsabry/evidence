# Prior-art reading

The reading list for Week 1 of the paper timeline. ~15 hours total. Fill in each entry as you go; the notes here become the related-work section.

**The goal of these reads is to find out whether the kernel still holds.** If any one of these papers makes the three-rule decomposition claim, the paper needs to pivot. The Week 1 success criterion is a confident "no, none of them does this" — not "I've read everything."

---

## ⚠️ First-pass status

The entries below were drafted by Claude (Anthropic Opus 4.7) from training-data
knowledge of each paper, with **no live web access**. They are **a working
baseline, not a substitute for reading the actual PDFs**. Before any draft of the
paper goes to a co-author, every entry tagged "first-pass" below must be
re-read against the source, exact quotes pulled, and citation strings
verified against the venue's authoritative form. Two specific risks:

1. **Stale framing.** Several of these papers had follow-ups in 2024–2025
   I may not have current knowledge of. Specifically: spot-check arXiv for
   newer extensions / replacements of Rashkin '23, Gao '23 (ALCE), and
   Liu '23 before locking the related-work section.
2. **Misattribution.** I'm reasonably confident on the *content* of each
   paper but less so on exact author lists and venue pages. Re-pull every
   citation from the venue or DBLP before submission.

The **verdict** at the bottom is mine to argue for; yours to ratify.

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

**Citation:** Rashkin, H., Nikolaev, V., Lamm, M., Aroyo, L., Collins, M., Das, D., Petrov, S., Tomar, G. S., Turc, I., Reitter, D. (2023). Measuring Attribution in Natural Language Generation Models. *TACL.*

**Status:** first-pass (Claude-drafted from training data; verify before citing).

**One-paragraph summary:** Introduces the AIS framework ("Attributable to Identified Sources"). A generation is AIS-attributable to a source iff "a generic hearer would, after reading the source, conclude that the generation is fully supported by the source." Operationalized as a two-stage human-evaluation protocol: first an interpretability check (is the generation a self-contained, interpretable statement?), then an attribution check (does the source support the interpretable form?). They report inter-annotator agreement and apply the framework to existing QA / summarization systems. The framework is the de-facto modern definition of "attribution" in the LLM literature.

**What it measures, in one bullet list:**
- Binary AIS judgment per (generation, source) pair from human raters.
- Inter-annotator agreement on AIS judgments.
- AIS rates of existing dialogue and QA systems.

**Where it conflates the three rules:**
- existence × in-context × support: **all three collapse into one judgment**. The "source" in AIS is the document set the rater is shown; whether the model actually had access to it during generation is out of scope. Existence isn't measured because the rater is shown the actual source. In-context isn't measured because the source presented to the rater IS the source by construction. Support is the whole signal.

**The gap it leaves for us:**
- AIS asks "does the source support the claim?" as a single binary. Our work asks the prior questions: "is the citation pointing at a real span (gate 1)? was that span actually visible to the model (gate 2)?" Both are pre-conditions AIS assumes away.

**Quotable lines for the related-work section:**
> *(first-pass: pull exact quotes from the published TACL version before citing)*

**Does this paper's claim subsume ours?**
**No** — but it's the dominant operational definition we're decomposing under. Position our work as: *"AIS gives a single attribution judgment; we decompose the judgment into three operational gates and show empirically that they catch distinct errors."*

---

### Bohnet et al., 2022 — "Attributed Question Answering"

**Citation:** Bohnet, B., Tran, V. Q., Verga, P., Aharoni, R., Andor, D., Soares, L. B., Eisenstein, J., Ganchev, K., Herzig, J., Hui, K., Kwiatkowski, T., Ma, J., Ni, J., Saralegui, T. S., Schuster, T., Cohen, W. W., Collins, M., Das, D., Metzler, D., Petrov, S., Webster, K. (2022). Attributed Question Answering: Evaluation and Modeling for Attributed Large Language Models. *arXiv:2212.08037.*

**Status:** first-pass (Claude-drafted).

**One-paragraph summary:** Defines the **attributed QA task**: a system must produce an answer + supporting passage(s) that attribute the answer. Compares architectural patterns (retrieve-then-generate, generate-then-retrieve, generate-then-rerank) on AIS-style human evaluations. Uses AIS as the evaluation protocol. The paper's contribution is task definition and architectural comparison, not citation-validation methodology.

**What it measures, in one bullet list:**
- Exact-match / F1 on answer correctness.
- AIS judgment on the produced (answer, evidence) pair (single binary signal).
- Retrieval recall + a few generation-quality metrics.

**Where it conflates the three rules:**
- existence × in-context × support: same single AIS judgment as Rashkin. The "evidence" the system produces is held against the gold corpus, so existence is trivially satisfied. The "did the model have access to the cited passage" question is implicit (assumed true) because they study retrieve-then-generate or rerank-then-generate systems where retrieved passages ARE what the model saw.

**The gap it leaves for us:**
- Defines the task we operate in but takes "the citation is good" as a single signal. Their failure analysis doesn't separate "wrong passage was selected" from "right passage doesn't support the claim."

**Quotable lines for the related-work section:**
> *(first-pass)*

**Does this paper's claim subsume ours?**
**No.** Defines the task; doesn't measure its decomposition.

---

### Gao et al., 2023 — "Enabling Large Language Models to Generate Text with Citations" (ALCE)

**Citation:** Gao, T., Yen, H., Yu, J., Chen, D. (2023). Enabling Large Language Models to Generate Text with Citations. *EMNLP.*

**Status:** first-pass (Claude-drafted). **Critical to verify** — this is our closest comparison.

**One-paragraph summary:** Introduces **ALCE**, the standard benchmark for generation-with-citations. Builds three datasets (ASQA from ambiguous QA, QAMPARI from list-style QA, ELI5 from long-form QA). Defines two automatic metrics: **citation precision** (the cited passage supports the statement) and **citation recall** (each statement has at least one supporting citation). Uses an NLI model (TRUE-style) to compute support automatically. The benchmark *assumes* citations point at retrieved passages — the model's output format is `[1][2]` references against the retrieved set the model was shown. Reports baselines across closed and open LLMs.

**What it measures, in one bullet list:**
- Citation precision (NLI-judged): does each cited passage entail the cited statement?
- Citation recall (NLI-judged): is each statement entailed by *some* citation?
- Answer correctness (exact match / claim recall).

**Where it conflates the three rules:**
- existence × in-context × support:
  - **Existence:** trivially satisfied by construction. Citations are `[N]` markers; if N exceeds the retrieved set, that's a parsing error treated as malformed, not measured as a rule.
  - **In-context:** assumed by construction. The "retrieved set" IS the citation domain. No mechanism for the model to cite outside it.
  - **Support:** their citation precision metric IS gate 3.
- ALCE measures gate 3 (well) and assumes gates 1 and 2 away. **The non-overlap question we ask cannot be asked under ALCE's setup** — there's no in-context failure to measure because the construction forbids it.

**The gap it leaves for us:**
- We test what happens when those construction assumptions don't hold — specifically, when the model produces citations that are syntactically well-formed but reach outside the retrieved set (gate 2 failure) or to non-existent IDs (gate 1 failure). ALCE doesn't ask whether real LLMs produce such failures; we measure it directly.

**Quotable lines for the related-work section:**
> *(first-pass; verify against EMNLP 2023 proceedings)*

**Does this paper's claim subsume ours?**
**Partially — and this is the most defensive cell of the matrix.** ALCE's citation precision metric IS our rule 3. But ALCE bakes rules 1 and 2 into its data construction, which means it can't reveal whether those rules contribute additional catch rate in deployment. Our decomposition is the explicit version of what ALCE smuggles into its setup. The pivot if reviewers push: position the paper as "ALCE measures rule 3 assuming rules 1 and 2 hold; we test whether the assumption holds in real LLM outputs and find [X]% of failures violate rules 1 or 2."

---

### Liu et al., 2023 — "Evaluating Verifiability in Generative Search Engines"

**Citation:** Liu, N. F., Zhang, T., Liang, P. (2023). Evaluating Verifiability in Generative Search Engines. *Findings of EMNLP.*

**Status:** first-pass (Claude-drafted).

**One-paragraph summary:** Audits commercial generative search engines (Bing Chat, NeevaAI, Perplexity, YouChat) on 1,500 questions across four sources. For each (statement, cited URL) pair, hires human raters to judge whether the citation supports the statement. Reports per-engine "citation precision" and "citation recall" + a "fluency" score. Finds a striking gap: high fluency, much lower verifiability. The closest spirit-of-the-paper to ours.

**What it measures, in one bullet list:**
- Citation precision (human-judged): the cited URL's content supports the statement.
- Citation recall (human-judged): each statement is supported by at least one cited URL.
- Fluency (Likert).

**Where it conflates the three rules:**
- existence × in-context × support:
  - **Existence:** raters report whether URLs resolve. Closest any prior paper gets to gate 1 — but it's a side observation, not the main metric.
  - **In-context:** not applicable to their setting. Search engines don't have a "retrieved set" disclosed; the model's training corpus IS its context. Their precision/recall doesn't distinguish "model saw this in retrieval" from "model recalled this from training."
  - **Support:** is the main metric.

**The gap it leaves for us:**
- They notice non-resolving URLs in passing; we measure the failure mode explicitly and show what percentage of *naturally-occurring* RAG hallucinations involve cited spans the model never read. They cannot ask the in-context question because their systems don't expose retrieval.

**Quotable lines for the related-work section:**
> *(first-pass; pull from Findings of EMNLP 2023)*

**Does this paper's claim subsume ours?**
**No.** Closest in spirit; complementary in scope. They study deployed search engines (closed systems, no separation of retrieval from generation); we study RAG (open systems with a measurable retrieved set).

---

## Adjacent (skim; one paragraph each)

### Menick et al., 2022 — "Teaching Language Models to Support Answers with Verified Quotes" (GopherCite)

**Citation:** Menick, J., Trebacz, M., Mikulik, V., Aslanides, J., Song, F., Chadwick, M., Glaese, M., Young, S., Campbell-Gillingham, L., Irving, G., McAleese, N. (2022). Teaching language models to support answers with verified quotes. *arXiv:2203.11147.* (DeepMind tech report.)

**Status:** first-pass (Claude-drafted).

**Summary + gap:** Trains an LLM via reinforcement learning to produce answer + verbatim quote from a retrieved document. The quote is *generated text* that the system then *verifies* against the retrieved document (substring check). Closest prior art on span-level claims. Crucially, the citation mechanism is **model emits a quote** vs. our **model points at a span we retrieved**. The substring-check is structurally similar to our gate 1 (existence in the document) plus a flavor of gate 2 (the quote was in the retrieved doc), but it's done over text generated by the model, not over span IDs the model was shown. Subsumes part of gate 2 ("did the model cite something from a retrieved source?") but does not separate it from support.

**Does it subsume ours?** **No.** Materially different mechanism (quote-emission vs. span-pointer). Worth a paragraph in §2 specifically because it shows that the field has been groping toward gate 2 for years without naming it.

---

### Kamalloo et al., 2023 — HAGRID

**Citation:** Kamalloo, E., Jafari, A., Zhang, X., Thakur, N., Lin, J. (2023). HAGRID: A Human-LLM Collaborative Dataset for Generative Information-Seeking with Attribution. *arXiv:2307.16883.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** A dataset of (query, retrieved-passages, LLM-drafted-then-human-edited answer with citations) for long-form QA. Reuses ALCE-style citation precision/recall metrics. **Same single-signal limitation as ALCE.** Doesn't subsume.

---

### Malaviya et al., 2024 — ExpertQA

**Citation:** Malaviya, C., Lee, S., Chen, S., Sieber, E., Yatskar, M., Roth, D. (2024). ExpertQA: Expert-Curated Questions and Attributed Answers. *NAACL.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** Expert-curated questions from professionals across 32 fields, with multi-attribute evaluations of LLM answers including factual correctness, source reliability, and attribution. Multi-dimensional evaluation framework, but the *attribution* dimension itself is a single rating ("does the citation support the claim"). Strong for domain coverage; uninstructive for our decomposition question.

---

### Gao et al., 2022 — RARR

**Citation:** Gao, L., Dai, Z., Pasupat, P., Chen, A., Chaganty, A. T., Fan, Y., Zhao, V. Y., Lao, N., Lee, H., Juan, D.-C., Guu, K. (2022). RARR: Researching and Revising What Language Models Say, Using Language Models. *ACL 2023.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** Post-hoc attribution: take an existing LLM output, search the web for evidence, edit the output to be supported. Operates on rule 3 (support) with a *revision* response rather than refusal. The deployment philosophy is the opposite of ours (rewrite vs. refuse). Methodologically useful as the polar opposite of our refuse-or-resolve framing; worth one sentence in §2.

---

### Asai et al., 2024 — Self-RAG

**Citation:** Asai, A., Wu, Z., Wang, Y., Sil, A., Hajishirzi, H. (2024). Self-RAG: Learning to Retrieve, Generate, and Critique through Self-Reflection. *ICLR.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** Trains a model to emit "reflection tokens" interspersed with generation: when to retrieve, whether retrieved passages are relevant, whether the output is supported. Internalizes a sort of rule-3 check via training. Doesn't *externalize* the rules as a validator; doesn't separate existence / in-context / support. The reflection tokens collapse all three into a single "is this supported?" signal at inference.

**Does it subsume ours?** **No** — but it's the most likely paper a reviewer will compare us against on architecture grounds. Defense: their approach trains the model to *self-grade*, which means failures are silent (the model believes its output). Our approach externalizes the check, which means failures are loud (the validator refuses). Different deployment story.

---

## Methodologically useful

### Es et al., 2023 — RAGAS

**Citation:** Es, S., James, J., Espinosa-Anke, L., Schockaert, S. (2023). RAGAS: Automated Evaluation of Retrieval Augmented Generation. *arXiv:2309.15217.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** Evaluation framework for RAG with three "metrics" (faithfulness, answer relevance, context relevance), each LLM-as-judge. **Faithfulness** is their version of gate 3. **Context relevance** is their version of "is retrieval relevant" (closer to retrieval evaluation than to gate 2). They don't ask "did the model cite outside the retrieved context" — assumed away. Useful precedent for "frameworks of RAG checks"; positions us as "the right granularity for frameworks like RAGAS to adopt."

---

### Honovich et al., 2022 — TRUE

**Citation:** Honovich, O., Aharoni, R., Herzig, J., Taitelbaum, H., Kukliansy, D., Cohen, V., Scialom, T., Szpektor, I., Hassidim, A., Matias, Y. (2022). TRUE: Re-evaluating Factual Consistency Evaluation. *NAACL 2022.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** The "NLI is the right paradigm" paper. Argues against QA-based factual-consistency metrics in favor of NLI cross-encoders. Sets the lineage we step into for gate 3. Doesn't address attribution structure; just measures entailment. Cite as the source of the NLI move; not subsuming.

---

### Gardner et al., 2020 — Contrast Sets

**Citation:** Gardner, M., Artzi, Y., Basmova, V., Berant, J., Bogin, B., Chen, S., Dasigi, P., Dua, D., Elazar, Y., Gottumukkala, A., Gupta, N., Hajishirzi, H., Ilharco, G., Khashabi, D., Lin, K., Liu, J., Liu, N. F., Mulcaire, P., Ning, Q., Singh, S., Smith, N. A., Subramanian, S., Tsarfaty, R., Wallace, E., Zhang, A., Zhou, B. (2020). Evaluating Models' Local Decision Boundaries via Contrast Sets. *Findings of EMNLP.*

**Status:** first-pass (Claude-drafted).

**Summary + gap:** Methodological paper. Argues that holding most of an example fixed and perturbing one aspect tests whether models learn the right decision boundary. We cite this as the justification for our failure-injection operators: each operator is a named perturbation applied to a known-good seed (`crates/evidence-eval/src/inject.rs`). Cite once in §4.4, take the methodology, move on.

---

## After all reads — the verdict

Filled in 2026-05-15 after first-pass reads (Claude-drafted; verify before publication):

```
Verdict: KERNEL HOLDS, with one defensive caveat.

The decomposition into existence / in-context / support is, to the best of my
training-data knowledge, not made explicit anywhere in this list. The
strongest existing approximations are:

  - AIS (Rashkin '23): one judgment, three sub-questions implicitly bundled.
  - ALCE (Gao '23): measures rule 3, bakes rules 1 and 2 into data construction.
  - Verifiability (Liu '23): observes URL non-resolution in passing; doesn't
    decompose.
  - GopherCite (Menick '22): closest to gate 2 mechanism, but via quote
    emission, not span-pointer validation.
  - Self-RAG (Asai '24): internalizes a single-signal support check;
    doesn't separate or externalize.

The defensive caveat: ALCE's "we assume citations are well-formed and from
the retrieved set" is exactly what our paper makes a measurable variable.
A determined reviewer can argue "your decomposition is the explicit form
of ALCE's data construction." Our defense is empirical: real LLMs violate
those assumed-away rules at a measurable rate, and the paper measures it.

Pivot triggers (none triggered in the first-pass reads):
  - If a published paper (anything I missed) explicitly names all three
    rules and measures them independently → kernel subsumed.
  - If a published paper measures rules 1+2+3 via one combined metric AND
    shows the metric's components are non-overlapping → kernel partially
    subsumed; we'd pivot to span-level granularity + refuse-or-resolve
    deployment philosophy as the main contribution.

Confidence: medium. This verdict is from training-data knowledge of papers
up to roughly mid-2024. 2025 RAG/attribution work needs a live arXiv scan
before lock. Three specific search queries to run before submission:
  1. "decompose attribution" OR "citation taxonomy" 2024..2025
  2. "in-context citation" OR "closed-set citation" 2024..2025
  3. Recent ALCE follow-ups; check Gao, Yen, Yu, Chen authors' subsequent work.
```
