# Citation Audit — `docs/paper/related-work.md`

**Scope.** Independent second-pass verification of every entry in the
reference list and every in-text citation in `docs/paper/related-work.md`.
Each entry was cross-checked against the canonical arXiv abstract page
and, where applicable, the ACL Anthology / OpenReview record.

**Legend.** ✅ confirmed verbatim · ⚠️ correction needed · ❌ unverifiable.

## Summary table

| # | Cited as | Claimed arXiv id | Status | Source(s) checked |
|---|---|---|---|---|
| 1 | Asai, Wu, Wang, Sil, Hajishirzi (2024). *Self-RAG: Learning to Retrieve, Generate, and Critique through Self-Reflection*. ICLR. | 2310.11511 | ✅ | arxiv.org/abs/2310.11511; OpenReview (ICLR 2024 oral) |
| 2 | Bohnet, Tran, Verga, Aharoni, Andor, Soares, Ciaramita, Eisenstein, Ganchev, Herzig, Hui, Kwiatkowski, Ma, Ni, Saralegui, Schuster, Cohen, Collins, Das, Metzler, Petrov, Webster (2022). *Attributed Question Answering: Evaluation and Modeling for Attributed Large Language Models*. | 2212.08037 | ✅ | arxiv.org/abs/2212.08037 |
| 3 | Choi, Guo, Fung, Wang (2025). *CiteGuard: Faithful Citation Attribution for LLMs via Retrieval-Augmented Validation*. (To appear, ACL 2026.) | 2510.17853 | ✅ | arxiv.org/abs/2510.17853 |
| 4 | Es, James, Espinosa-Anke, Schockaert (2024). *RAGAs: Automated Evaluation of Retrieval Augmented Generation*. EACL Demo, 150–158. | 2309.15217 | ✅ | arxiv.org/abs/2309.15217; aclanthology.org/2024.eacl-demo.16 |
| 5 | Gao, Dai, Pasupat, Chen, Chaganty, Fan, Zhao, Lao, Lee, Juan, Guu (2023). *RARR: Researching and Revising What Language Models Say, Using Language Models*. ACL, 16477–16508. | 2210.08726 | ✅ | arxiv.org/abs/2210.08726; aclanthology.org/2023.acl-long.910 |
| 6 | Gao, Yen, Yu, Chen (2023). *Enabling Large Language Models to Generate Text with Citations*. EMNLP, 6465–6488. | 2305.14627 | ✅ | arxiv.org/abs/2305.14627; aclanthology.org/2023.emnlp-main.398 |
| 7 | Gardner, Artzi, **Basmova**, Berant, Bogin, Chen, Dasigi, Dua, Elazar, Gottumukkala, Gupta, Hajishirzi, Ilharco, Khashabi, Lin, Liu, Liu, Mulcaire, Ning, Singh, Smith, Subramanian, Tsarfaty, Wallace, Zhang, Zhou (2020). *Evaluating Models' Local Decision Boundaries via Contrast Sets*. Findings EMNLP, 1307–1323. | — | ⚠️ | aclanthology.org/2020.findings-emnlp.117 |
| 8 | Honovich, Aharoni, Herzig, Taitelbaum, Kukliansy, Cohen, Scialom, Szpektor, Hassidim, Matias (2022). *TRUE: Re-evaluating Factual Consistency Evaluation*. NAACL-HLT, 3905–3920. | 2204.04991 | ✅ | arxiv.org/abs/2204.04991; aclanthology.org/2022.naacl-main.287 |
| 9 | Kamalloo, Jafari, Zhang, Thakur, Lin (2023). *HAGRID: A Human-LLM Collaborative Dataset for Generative Information-Seeking with Attribution*. | 2307.16883 | ✅ | arxiv.org/abs/2307.16883 |
| 10 | Liu, Zhang, Liang (2023). *Evaluating Verifiability in Generative Search Engines*. Findings EMNLP, 7001–7025. | 2304.09848 | ✅ | arxiv.org/abs/2304.09848; aclanthology.org/2023.findings-emnlp.467 |
| 11 | Malaviya, Lee, Chen, Sieber, Yatskar, Roth (2024). *ExpertQA: Expert-Curated Questions and Attributed Answers*. NAACL-HLT, 3025–3045. | 2309.07852 | ✅ | arxiv.org/abs/2309.07852; aclanthology.org/2024.naacl-long.167 |
| 12 | Menick, Trebacz, Mikulik, Aslanides, Song, Chadwick, Glaese, Young, Campbell-Gillingham, Irving, McAleese (2022). *Teaching Language Models to Support Answers with Verified Quotes*. | 2203.11147 | ✅ | arxiv.org/abs/2203.11147 |
| 13 | Rashkin, Nikolaev, Lamm, Aroyo, Collins, Das, Petrov, Tomar, Turc, Reitter (2023). *Measuring Attribution in Natural Language Generation Models*. *Computational Linguistics* 49(4), 777–840. | 2112.12870 | ✅ | arxiv.org/abs/2112.12870; aclanthology.org/2023.cl-4.2 |
| 14 | Wallat, Heuss, de Rijke, Anand (2024). *Correctness is not Faithfulness in RAG Attributions*. | 2412.18004 | ✅ | arxiv.org/abs/2412.18004 |

**Totals.** 14 reference-list entries checked. ✅ 13 · ⚠️ 1 · ❌ 0.

In-text `\citep{...}`-style citations (Rashkin 2023, Bohnet 2022, Honovich
2022, Gao 2023 [ALCE], Kamalloo 2023, Malaviya 2024, Menick 2022, Liu 2023,
Gao 2023 [RARR], Asai 2024, Es 2023, Gardner 2020, Choi 2025, Wallat 2024)
all resolve to the reference-list entries above. The in-text year for Es et
al. is "2023" in §"Frameworks and methodology"; the reference list entry
uses "2024" (the EACL Demo publication year). This is a minor in-text
year/reference-list mismatch — see appendix.

## Corrections to apply

### C1 (⚠️) Gardner et al. (2020): author surname misspelling

**Where.** Reference list entry, line 116 of `docs/paper/related-work.md`.

**Issue.** The third author is listed as **"Basmova"**. The ACL Anthology
record (`aclanthology.org/2020.findings-emnlp.117`) spells the name
**"Basmov"** (no trailing "a"). The same arXiv-tracked variant
(arXiv:2004.02709) also uses "Basmov". This is a spelling error, not a
fabrication.

**Corrected entry (verbatim, drop the trailing "a"):**

```
- Gardner, M., Artzi, Y., Basmov, V., Berant, J., Bogin, B., Chen, S.,
  Dasigi, P., Dua, D., Elazar, Y., Gottumukkala, A., Gupta, N., Hajishirzi,
  H., Ilharco, G., Khashabi, D., Lin, K., Liu, J., Liu, N. F., Mulcaire,
  P., Ning, Q., Singh, S., Smith, N. A., Subramanian, S., Tsarfaty, R.,
  Wallace, E., Zhang, A., & Zhou, B. (2020). Evaluating Models' Local
  Decision Boundaries via Contrast Sets. *Findings of the Association
  for Computational Linguistics: EMNLP 2020*, 1307–1323.
```

### C2 (cosmetic, optional) Es et al. in-text year vs. reference list

**Where.** In-text citation on line 54 ("RAGAS (Es et al., 2023)") vs.
reference-list entry on line 100 dated 2024.

**Issue.** The arXiv preprint is dated 2023 (submitted Sept 2023); the EACL
2024 System Demonstrations publication is dated 2024. The reference list
uses the formal publication year (2024), but the in-text citation uses
2023. Both are technically defensible, but they should match.

**Suggested fix.** Update the in-text citation to "Es et al. (2024)" to
match the reference-list year. Not a factual error — purely a consistency
nit.

## Notes on prior-pass flagged entries

All entries the prior pass touched are verified ✅ with no further issues:

- **Wallat, Heuss, de Rijke, Anand (2024), arXiv:2412.18004.** Authors,
  title, year, and arXiv id all confirmed verbatim against
  `arxiv.org/abs/2412.18004`. Title is exactly "Correctness is not
  Faithfulness in RAG Attributions".
- **Choi, Guo, Fung, Wang (2025), CiteGuard, arXiv:2510.17853.** Authors,
  title, and arXiv id confirmed. The "(To appear, ACL 2026.)" annotation
  is consistent with the arXiv landing page, which reports ACL 2026 Main
  Conference acceptance. First-author given name on arXiv is "Yee Man
  Choi" — the reference-list initials "Y. M." are correct.
- **Gao et al. (2023), RARR, arXiv:2210.08726.** Author list, ACL 2023
  venue, and page range 16477–16508 all confirmed.
- **Gao et al. (2023), ALCE, arXiv:2305.14627.** Author list, EMNLP 2023
  venue, and page range 6465–6488 all confirmed.
- **Asai et al. (2024), Self-RAG, arXiv:2310.11511.** ICLR 2024 venue
  confirmed via OpenReview (oral). Author order matches.
- **Rashkin et al. (2023), arXiv:2112.12870.** *Computational Linguistics*
  49(4) pp. 777–840 confirmed via `aclanthology.org/2023.cl-4.2`. arXiv
  submission predates publication; the "2023" year refers to the journal
  publication, which is the correct primary citation form.
- **Menick et al. (2022), GopherCite, arXiv:2203.11147.** Authors and
  title confirmed. (Note: arXiv title is rendered "Teaching language
  models to support answers with verified quotes" — lowercase; the
  related-work entry uses title case, which is a conventional reference
  style and not an error.)
- **Honovich et al. (2022), TRUE, arXiv:2204.04991.** NAACL 2022 pp.
  3905–3920 confirmed.
- **Kamalloo et al. (2023), HAGRID, arXiv:2307.16883.** Author order,
  title, year confirmed (arXiv preprint, no formal venue claimed).
- **Malaviya et al. (2024), ExpertQA, arXiv:2309.07852.** NAACL 2024 pp.
  3025–3045 confirmed.
- **Es et al. (2024), RAGAs, arXiv:2309.15217.** EACL 2024 System
  Demonstrations pp. 150–158 confirmed. Surname rendered as
  "Espinosa-Anke" in the reference list and "Espinosa Anke" on the ACL
  Anthology page; the hyphenated form matches the arXiv author listing
  and is the author's preferred rendering — not a correction.
- **Bohnet et al. (2022), Attributed QA, arXiv:2212.08037.** All 22
  authors confirmed in correct order.
- **Liu et al. (2023), arXiv:2304.09848.** Findings of EMNLP 2023 pp.
  7001–7025 confirmed.

## Bottom line

No fabricated entries detected on this pass. One spelling error
("Basmova" → "Basmov") and one minor in-text/reference year inconsistency
(Es et al. 2023 vs. 2024) are the only items requiring action.
