# Related-work matrix

Fill in after the Week 1 reads. The point of this table is to make the "gap paragraph" in §2 of the paper concrete: for each prior paper, mark which of our three rules they measure, which they conflate, and which they don't address.

A clean reading should leave at least one row where no prior paper marks **all three** as "measures cleanly." That's the empirical gap. If every row is solid blue (all three measured), the paper needs to pivot.

> **First-pass status (2026-05-15):** the matrix below was filled in by Claude from training-data knowledge of each paper, paired with the entries in [`prior-art-reading.md`](prior-art-reading.md). Verify each cell against the source before citing.

---

## Legend

- ✅ measures cleanly — paper has a dedicated metric or signal for this rule
- 🔶 conflates — paper's metric covers this rule but bundled with at least one other
- ⬜ does not address — paper's scope doesn't include this rule
- 🚫 assumes away — paper bakes the rule into its data construction so it can't be measured

---

## The matrix (first-pass)

| Paper | Existence | In-context | Support | Notes |
|---|---|---|---|---|
| **Rashkin '23 (AIS)** | ⬜ | 🔶 | 🔶 | Single AIS judgment bundles in-context + support. Existence assumed by giving the rater the actual source. |
| **Bohnet '22 (Attributed QA)** | ⬜ | 🔶 | 🔶 | Defines task; uses AIS for eval → same bundling. |
| **Gao '23 (ALCE)** | 🚫 | 🚫 | ✅ | Citation precision = our rule 3. Rules 1 + 2 baked into construction (citations are `[N]` markers against the retrieved set). |
| **Liu '23 (verifiability)** | 🔶 | ⬜ | ✅ | Citation precision/recall = our rule 3. URL non-resolution noted in passing (closest any paper gets to existence). In-context inapplicable (search engines don't expose retrieval). |
| **Menick '22 (GopherCite)** | 🔶 | 🔶 | ⬜ | Verifies a model-emitted quote is a substring of a retrieved doc. Touches gates 1 + 2 via the substring check but doesn't separate or measure support. |
| **Kamalloo '23 (HAGRID)** | 🚫 | 🚫 | ✅ | Same ALCE-derived metric. |
| **Malaviya '24 (ExpertQA)** | ⬜ | ⬜ | 🔶 | Multi-attribute eval; attribution is one of N dimensions, rated as one signal. |
| **Gao '22 (RARR)** | ⬜ | ⬜ | ✅ | Post-hoc revision against retrieved evidence. Rule 3 with a *rewrite* response. |
| **Asai '24 (Self-RAG)** | ⬜ | ⬜ | 🔶 | Reflection tokens emit a single "is this supported" signal at inference time. |
| **Es '23 (RAGAS)** | ⬜ | 🔶 | ✅ | Faithfulness = rule 3. Context relevance is retrieval evaluation, not gate 2. |
| **Honovich '22 (TRUE)** | ⬜ | ⬜ | ✅ | NLI for faithfulness; orthogonal to attribution structure. |

---

## What the pattern shows

The first-pass pattern is:

- **Existence: universally ⬜ or 🔶.** No paper makes existence its own measurement. The closest is Liu '23 noticing URL non-resolution.
- **In-context: universally 🔶, 🚫, or ⬜.** ALCE / HAGRID bake it away. AIS / Bohnet bundle it into the single attribution judgment. RAGAS treats it as retrieval evaluation. No paper measures it as an independent failure mode of generation.
- **Support: well-traveled with ✅ in most rows.** This is the dominant axis in the field.

**This matches the hypothesis in the original related-work-table template.** The gap is real. The paper's empirical contribution is to measure existence and in-context as independent signals and show they catch errors that support-alone misses.

---

## The gap paragraph (draft, post-reads)

Draft prose for §2, to be polished before submission:

> Prior work on RAG citation correctness can be organized along three axes: whether the cited span exists in the corpus (gate 1), whether it was in the retrieval set the model saw for the query (gate 2), and whether it supports the claim (gate 3). The dominant operational definition, AIS (Rashkin et al., 2023), bundles all three into a single human-judged signal. ALCE (Gao et al., 2023) and its derivatives (Kamalloo et al., 2023) measure gate 3 well but assume gates 1 and 2 away by construction — citations are restricted to integer markers against the retrieved set. Liu et al. (2023) audit commercial search engines for citation verifiability and note URL non-resolution in passing but cannot measure in-context grounding (search engines don't expose retrieval). GopherCite (Menick et al., 2022) verifies model-emitted quotes against retrieved documents via substring checks, touching gates 1 and 2 mechanically but not separating them from support. Self-RAG (Asai et al., 2024) internalizes a single "is this supported" signal through reflection tokens trained at fine-tuning time. To our knowledge, no prior work measures the three gates as independent signals at inference time, and no prior work demonstrates that they catch disjoint classes of errors. This paper does both.

---

## Pivot triggers (none triggered in the first-pass)

The verdict from [`prior-art-reading.md`](prior-art-reading.md) names two:

1. If a missed paper explicitly names all three rules and measures them independently → **kernel subsumed**; restart claims doc.
2. If a missed paper measures rules 1+2+3 via one combined metric AND shows the metric's components are non-overlapping → **kernel partially subsumed**; pivot to span-level granularity + refuse-or-resolve deployment philosophy.

**Action items before lock:**
- Verify every cell against the actual paper, not the training-data summary.
- Run the three arXiv search queries listed in the verdict block.
- Send the matrix + gap paragraph to one trusted reader for the "where does this overclaim" check.
