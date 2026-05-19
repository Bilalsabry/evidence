# 2 Related Work

**Attribution and faithfulness.** The dominant operational definition of
attribution is AIS (Rashkin et al., 2023), which judges whether a model
statement is fully derivable from an identified source via a two-stage
human protocol (interpretability, then an attribution rating). AIS bundles
all three of our constraints into a single human judgment: the source is
shown to the rater, so existence and in-context grounding are satisfied
tautologically and only support is genuinely assessed. Attributed QA
(Bohnet et al., 2022) defines the attributed-answering task and benchmarks
retrieve-then-generate variants, but reuses AIS for evaluation and inherits
its bundling. TRUE (Honovich et al., 2022) standardizes factual-consistency
evaluation and finds large-scale NLI and question-generation-and-answering
to be strong and complementary; it establishes the NLI-for-faithfulness
lineage our support gate draws on but is orthogonal to attribution
decomposition.

**RAG citation benchmarks.** ALCE (Gao et al., 2023) is the standard
generation-with-citations benchmark; its redundancy-aware citation
precision measures support carefully via an NLI model, but citations are
integer indices into the retrieved set by construction
($c_{i,j} \in D$), so existence and in-context grounding are *assumed true
by construction and cannot be measured*. HAGRID (Kamalloo et al., 2023)
reuses the ALCE-style metric on a human-LLM collaborative corpus and
carries the same single-signal limitation. ExpertQA (Malaviya et al.,
2024) provides expert-curated questions with multi-attribute evaluation,
but attribution is one bundled dimension among several and is not itself
decomposed into existence, in-context, and support.

**Span-level and constrained citing.** GopherCite (Menick et al., 2022)
trains a model to answer with verbatim quotes and *enforces*
quote-document fidelity through constrained decoding rather than measuring
it: existence and in-context grounding cannot fail by construction, and
its primary "Supported & Plausible" metric bundles support with
plausibility. It shows the field has groped toward gate-1/gate-2
enforcement without naming it as a separable, measurable concern.

**Verifiability and post-hoc revision.** Liu et al. (2023) audit
commercial generative search engines and report that only about half of
generated sentences are fully supported by their citations; it is closest
in spirit to this work but notes URL non-resolution only in passing and
cannot measure in-context grounding, since deployed search engines do not
expose a retrieved set. RARR (Gao et al., 2023) [verify] post-edits model
output to fix unsupported content, operating on support with
revise-the-output semantics — the inverse of our refuse-or-resolve
deployment philosophy. Self-RAG (Asai et al., 2024) is the closest
existing decomposition: it trains reflection tokens that separately rate
passage relevance (`IsRel`) and three-valued support (`IsSup`). But
`IsRel` measures retrieval *relevance*, not whether a cited span was in
the prompt for *this* query, so it addresses neither existence nor our
in-context gate; and both signals are *internalized* trained tokens, not
*externalized* validator gates that refuse the output.

**Frameworks and methodology.** RAGAS (Es et al., 2023) is a
reference-free RAG evaluation framework whose faithfulness metric is
essentially our support gate via LLM-as-judge, while its context-relevance
metric evaluates retrieval rather than citation grounding; we position the
three-rule decomposition as the right granularity for RAGAS-shaped tools
to adopt. The synthetic failure-injection design follows the contrast-set
methodology of Gardner et al. (2020). Recent agentic-citation work is
adjacent but disjoint: CiteGuard (Choi et al., 2026) [verify] reframes
citation evaluation as predicting which academic paper a human would cite
(the CiteME task), a recommendation problem rather than grounding
validation; and mechanistic accounts of citation hallucination as
attention/feed-forward coordination failure (Dassen et al., 2026)
[verify] operate inside the transformer to explain *why* failures occur,
complementary to our validator-boundary question of *which* failure
occurred.

**The gap.** No prior work separates existence, in-context, and support as
three independently *measured* gates at the validator boundary, and none
demonstrates that they catch distinct, non-overlapping classes of citation
errors. The closest near-misses either bundle the three into one judgment
(AIS), assume two of them away by construction (ALCE, GopherCite),
internalize a partial decomposition as trained signals (Self-RAG), or
study a setting where in-context grounding is unmeasurable (Liu et al.,
2023). This paper measures all three as external, refusing gates and shows
their catch sets are disjoint on the benchmark. We claim citation
*correctness* at the validator boundary, not causal faithfulness: the
in-context gate shows a span was *available* to the model, not that the
model *relied* on it, and post-rationalization remains possible
(cf. Dassen et al., 2026 [verify]) — causal reliance is a separate,
out-of-scope dimension.
