# Related-work matrix

For each prior paper, mark which of our three rules they measure, which they conflate, and which they don't address. The matrix is what makes the "gap paragraph" in §2 of the paper concrete.

> **Status (2026-05-15):** web-verified against arXiv abstracts + HTML renders. 🔴 cells changed materially from the first-pass. The human read should pay extra attention to the 🔴 cells before submission.

---

## Legend

- ✅ measures cleanly — paper has a dedicated metric or signal for this rule
- 🔶 conflates — paper's metric covers this rule but bundled with at least one other
- ⬜ does not address — paper's scope doesn't include this rule
- 🚫 assumes away — paper bakes the rule into its data construction so it can't be measured
- 🔴 — first-pass entry was materially wrong; web-verified corrected it

---

## The matrix

| Paper | Existence | In-context | Support | Notes |
|---|---|---|---|---|
| **Rashkin '23 (AIS)** | ⬜ | 🔶 | 🔶 | Single AIS judgment bundles support; source is assumed given (existence trivial). Two-stage protocol (Interpretability + AIS Rating) is shape + support, not gates 1+2+3. |
| **Bohnet '22 (Attributed QA)** | ⬜ | 🔶 | 🔶 | Defines the task; uses AIS for eval → inherits its bundling. |
| **Gao '23 (ALCE)** | 🚫 | 🚫 | ✅ 🔴 | Citation precision is now confirmed to include a **redundancy guard** (a cite is "irrelevant" if it alone can't support AND removing it doesn't change overall support). Stricter than first-pass credited. Rules 1+2 baked in by construction (`c_{i,j} ∈ D`). |
| **Liu '23 (verifiability)** | 🔶 | ⬜ | ✅ | URL non-resolution observed in passing; not a primary metric. In-context inapplicable (commercial search engines don't expose retrieval). |
| **Menick '22 (GopherCite)** | 🚫 🔴 | 🚫 🔴 | 🔶 | Constrained decoding *forces* gates 1+2 by construction (corrected from first-pass "substring verification"). Primary metric S&P bundles support with plausibility. |
| **Kamalloo '23 (HAGRID)** | 🚫 | 🚫 | ✅ | Same ALCE-derived metric on a collaboratively-built corpus. |
| **Malaviya '24 (ExpertQA)** | ⬜ | ⬜ | 🔶 | Multi-attribute expert eval; attribution one of several dimensions, internally bundled with factuality. |
| **Gao '22 (RARR)** | ⬜ | ⬜ | ✅ | Post-hoc revision against retrieved evidence. Rule 3 with rewrite-response semantics (opposite of our refuse-or-resolve). |
| **Asai '24 (Self-RAG)** | ⬜ | ⬜ 🔴 | 🔶 🔴 | More decomposed than first-pass credited: has `IsRel` (passage relevance) + `IsSup` (3-valued support). But IsRel ≠ our gate 2 (it's retrieval-quality, not citation-in-context); both signals are *internalized* (trained tokens), not externalized validator gates. |
| **Es '23 (RAGAS)** | ⬜ | ⬜ | ✅ | Faithfulness ≈ our gate 3 via LLM-as-judge. Context relevance evaluates retrieval, not citation gates. |
| **Honovich '22 (TRUE)** | ⬜ | ⬜ | ✅ | NLI as one of two complementary strong methods (NLI + QG-and-QA). Orthogonal to attribution decomposition. |

### 2024–2026 additions (from web-search lock pass)

| Paper | Existence | In-context | Support | Notes |
|---|---|---|---|---|
| **Zhao '26 (RAG hallucination survey)** | ⬜ | ⬜ | ⬜ | Different abstraction: taxonomy of *hallucination types* (Outdatedness, Overconfidence, etc.), not citation-validation gates. Surveys mitigation techniques. |
| **Choi '26 (CiteGuard)** | ⬜ | ⬜ | ⬜ | Different task: academic citation attribution (CiteME). "Did the model cite the same paper a human would have." Not citation-grounding validation. |
| **Dassen '26 (FACTUM)** | ⬜ | ⬜ | ⬜ | Different layer: mechanistic interpretability (attention/FFN coordination), not validation gates. |

---

## What the pattern shows (post-verification)

The matrix retains the original pattern's shape, with the corrections noted above:

- **Existence: universally ⬜, 🔶, or 🚫.** Nobody measures existence as a primary signal. The closest is Liu '23 (URL non-resolution in passing) and the two construction-bound papers (ALCE, GopherCite) that satisfy existence by data-shape contract.
- **In-context: universally 🔶, 🚫, ⬜, or — for Self-RAG — partially decomposed via `IsRel`.** No paper measures the explicit "did the model cite a span we showed it for this query" gate.
- **Support: well-traveled with ✅ in most rows.** Established axis.

**The gap is real, and a little tighter than the first-pass claimed.** Self-RAG's `IsRel` + `IsSup` is the closest existing decomposition; our framing needs to acknowledge that and articulate why externalized gates + closed-set citation are different from internalized reflection tokens.

---

## The gap paragraph (post-verification draft)

Draft prose for §2:

> Prior work on RAG citation correctness can be organized along three axes: whether the cited span exists in the corpus (gate 1), whether it was in the retrieval set the model saw for the query (gate 2), and whether it supports the claim (gate 3). The dominant operational definition, AIS (Rashkin et al., 2023), bundles all three into a single human-judged signal. ALCE (Gao et al., 2023) and its derivatives (Kamalloo et al., 2023) measure gate 3 carefully — with a redundancy-aware precision metric — but assume gates 1 and 2 away by construction: citations are restricted to integer markers against the retrieved set. Liu et al. (2023) audit commercial search engines for citation verifiability and note URL non-resolution in passing but cannot measure in-context grounding (search engines don't expose retrieval). GopherCite (Menick et al., 2022) forces gates 1 and 2 by constrained decoding rather than measuring them. Self-RAG (Asai et al., 2024) is the closest existing decomposition: separate reflection tokens for retrieval relevance (`IsRel`) and support (`IsSup`); but its signals are trained inside the model rather than externalized as validator gates that refuse output, and neither token addresses gate 2 (whether the cited passage was in the prompt the model received for this query). To our knowledge, no prior work measures the three gates as independent signals at inference time at the validator boundary, and no prior work demonstrates that they catch disjoint classes of errors. This paper does both.

---

## Pivot triggers (none fired)

The verdict from [`prior-art-reading.md`](prior-art-reading.md) names two triggers; neither fired in the verification pass:

1. If a missed paper explicitly names all three rules and measures them independently → **kernel subsumed**.
2. If a missed paper measures rules 1+2+3 via one combined metric AND shows non-overlap → **kernel partially subsumed**; pivot to span-level granularity + refuse-or-resolve deployment philosophy.

The Self-RAG `IsRel`+`IsSup` finding is the closest to a (partial) trigger — but neither token is gate 2 in our sense, and the externalization difference is real enough to defend.

**Pre-submission action items:**
- Verify each cell against the source PDF (not just the abstract / HTML render).
- Pull exact quotations from the venue's authoritative typeset before citing.
- Re-run the three search queries closer to submission (the 1,200-papers-per-year field will move).
- Send the gap paragraph to one trusted external reader for the "where does this overclaim" check.
