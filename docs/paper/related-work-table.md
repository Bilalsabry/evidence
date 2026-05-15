# Related-work matrix

Fill in after the Week 1 reads. The point of this table is to make the "gap paragraph" in §2 of the paper concrete: for each prior paper, mark which of our three rules they measure, which they conflate, and which they don't address.

A clean reading should leave at least one row where no prior paper marks **all three** as "measures cleanly." That's the empirical gap. If every row is solid blue (all three measured), the paper needs to pivot.

---

## Legend

- ✅ measures cleanly — paper has a dedicated metric or signal for this rule
- 🔶 conflates — paper's metric covers this rule but bundled with at least one other
- ⬜ does not address — paper's scope doesn't include this

---

## The matrix

| Paper | Existence | In-context | Support | Notes / where each rule lives in that paper |
|---|---|---|---|---|
| Rashkin '23 (AIS) | ? | ? | ? | (fill in) |
| Bohnet '22 (Attributed QA) | ? | ? | ? | (fill in) |
| Gao '23 (ALCE) | ? | ? | ? | (fill in) |
| Liu '23 (verifiability) | ? | ? | ? | (fill in) |
| Menick '22 (GopherCite) | ? | ? | ? | (fill in) |
| Kamalloo '23 (HAGRID) | ? | ? | ? | (fill in) |
| Malaviya '24 (ExpertQA) | ? | ? | ? | (fill in) |
| Gao '22 (RARR) | ? | ? | ? | (fill in) |
| Asai '24 (Self-RAG) | ? | ? | ? | (fill in) |
| Es '23 (RAGAS) | ? | ? | ? | (fill in) |
| Honovich '22 (TRUE) | ? | ? | ? | (fill in) |

---

## The expected pattern (hypothesis, before reads)

This is what we expect to find. The reads either confirm it or kill the kernel.

- **Existence is almost universally ⬜.** Most papers assume citations are well-formed by construction (they generate text with citation markers and don't check the markers point to real spans). The few that do (likely Liu '23, GopherCite) treat it as a tokenization issue.
- **In-context is the conflated cell.** Many papers (Rashkin, Gao '23, RAGAS) include in-context implicitly under "is the citation related to the retrieved context" but don't measure it as separate from support.
- **Support is the well-measured cell.** Every attribution paper measures some version of "does the cited content entail the claim." This is the well-traveled lineage from Honovich '22 forward.

If the table looks like:

| ... | ⬜ | 🔶 | ✅ |
| ... | ⬜ | 🔶 | ✅ |
| ... | ⬜ | 🔶 | ✅ |

...then the kernel is real and the contribution is "we separate the three columns." If any row is `✅ ✅ ✅`, that row is the paper that subsumes ours and we pivot.

---

## What the gap paragraph reads like once filled in

The draft template for §2 of the paper, to be revised once the table is real:

> Prior work on RAG citation correctness can be organized along three axes: whether the cited span exists in the corpus, whether it was in the retrieval set the model saw, and whether it supports the claim. Existing benchmarks tend to either (a) collapse all three into a single "is this citation good?" signal (e.g., ALCE, RAGAS), (b) measure only support via NLI-style entailment (e.g., TRUE, FActScore, ExpertQA), or (c) measure verifiability without separating in-context from support (Liu '23). To our knowledge, no prior work measures the three rules as independent signals, and no prior work demonstrates that they catch disjoint classes of errors. This paper does both.

If the table fills in and that paragraph doesn't survive — pivot.
