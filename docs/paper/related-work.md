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
expose a retrieved set. RARR (Gao et al., 2023) post-edits model
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
adjacent but disjoint: CiteGuard (Choi et al., 2025) reframes
citation evaluation as predicting which academic paper a human would cite
(the CiteME task), a recommendation problem rather than grounding
validation; and accounts of citation hallucination as
post-rationalization rather than genuine reliance (Wallat et al., 2024)
operate on the model's reasoning to explain *why* failures occur,
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
(cf. Wallat et al., 2024) — causal reliance is a separate,
out-of-scope dimension.

## References

- Asai, A., Wu, Z., Wang, Y., Sil, A., & Hajishirzi, H. (2024).
  Self-RAG: Learning to Retrieve, Generate, and Critique through
  Self-Reflection. *International Conference on Learning Representations
  (ICLR)*. arXiv:2310.11511.
- Bohnet, B., Tran, V. Q., Verga, P., Aharoni, R., Andor, D., Soares,
  L. B., Ciaramita, M., Eisenstein, J., Ganchev, K., Herzig, J., Hui,
  K., Kwiatkowski, T., Ma, J., Ni, J., Saralegui, L. S., Schuster, T.,
  Cohen, W. W., Collins, M., Das, D., Metzler, D., Petrov, S., &
  Webster, K. (2022). Attributed Question Answering: Evaluation and
  Modeling for Attributed Large Language Models. arXiv:2212.08037.
- Choi, Y. M., Guo, X., Fung, Y. R., & Wang, Q. (2025). CiteGuard:
  Faithful Citation Attribution for LLMs via Retrieval-Augmented
  Validation. arXiv:2510.17853. (To appear, ACL 2026.)
- Es, S., James, J., Espinosa-Anke, L., & Schockaert, S. (2024). RAGAs:
  Automated Evaluation of Retrieval Augmented Generation. *Proceedings
  of the 18th Conference of the European Chapter of the Association for
  Computational Linguistics (EACL): System Demonstrations*, 150–158.
  arXiv:2309.15217.
- Gao, L., Dai, Z., Pasupat, P., Chen, A., Chaganty, A. T., Fan, Y.,
  Zhao, V., Lao, N., Lee, H., Juan, D.-C., & Guu, K. (2023). RARR:
  Researching and Revising What Language Models Say, Using Language
  Models. *Proceedings of the 61st Annual Meeting of the Association
  for Computational Linguistics (ACL)*, 16477–16508. arXiv:2210.08726.
- Gao, T., Yen, H., Yu, J., & Chen, D. (2023). Enabling Large Language
  Models to Generate Text with Citations. *Proceedings of the 2023
  Conference on Empirical Methods in Natural Language Processing
  (EMNLP)*, 6465–6488. arXiv:2305.14627.
- Gardner, M., Artzi, Y., Basmova, V., Berant, J., Bogin, B., Chen, S.,
  Dasigi, P., Dua, D., Elazar, Y., Gottumukkala, A., Gupta, N., Hajishirzi,
  H., Ilharco, G., Khashabi, D., Lin, K., Liu, J., Liu, N. F., Mulcaire,
  P., Ning, Q., Singh, S., Smith, N. A., Subramanian, S., Tsarfaty, R.,
  Wallace, E., Zhang, A., & Zhou, B. (2020). Evaluating Models' Local
  Decision Boundaries via Contrast Sets. *Findings of the Association
  for Computational Linguistics: EMNLP 2020*, 1307–1323.
- Honovich, O., Aharoni, R., Herzig, J., Taitelbaum, H., Kukliansy, D.,
  Cohen, V., Scialom, T., Szpektor, I., Hassidim, A., & Matias, Y.
  (2022). TRUE: Re-evaluating Factual Consistency Evaluation.
  *Proceedings of the 2022 Conference of the North American Chapter of
  the Association for Computational Linguistics: Human Language
  Technologies (NAACL-HLT)*, 3905–3920. arXiv:2204.04991.
- Kamalloo, E., Jafari, A., Zhang, X., Thakur, N., & Lin, J. (2023).
  HAGRID: A Human-LLM Collaborative Dataset for Generative
  Information-Seeking with Attribution. arXiv:2307.16883.
- Liu, N. F., Zhang, T., & Liang, P. (2023). Evaluating Verifiability in
  Generative Search Engines. *Findings of the Association for
  Computational Linguistics: EMNLP 2023*, 7001–7025. arXiv:2304.09848.
- Malaviya, C., Lee, S., Chen, S., Sieber, E., Yatskar, M., & Roth, D.
  (2024). ExpertQA: Expert-Curated Questions and Attributed Answers.
  *Proceedings of the 2024 Conference of the North American Chapter of
  the Association for Computational Linguistics: Human Language
  Technologies (NAACL-HLT)*, 3025–3045. arXiv:2309.07852.
- Menick, J., Trebacz, M., Mikulik, V., Aslanides, J., Song, F.,
  Chadwick, M., Glaese, M., Young, S., Campbell-Gillingham, L., Irving,
  G., & McAleese, N. (2022). Teaching Language Models to Support
  Answers with Verified Quotes. arXiv:2203.11147.
- Rashkin, H., Nikolaev, V., Lamm, M., Aroyo, L., Collins, M., Das, D.,
  Petrov, S., Tomar, G. S., Turc, I., & Reitter, D. (2023). Measuring
  Attribution in Natural Language Generation Models. *Computational
  Linguistics*, 49(4), 777–840. arXiv:2112.12870.
- Wallat, J., Heuss, M., de Rijke, M., & Anand, A. (2024). Correctness
  is not Faithfulness in RAG Attributions. arXiv:2412.18004.
