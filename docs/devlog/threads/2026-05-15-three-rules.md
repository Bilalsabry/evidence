# Thread: Three rules that make AI citations trustworthy

*Companion to the [dev log](../2026-05-15-three-rules.md). Each post ≤ 280 chars to fit on both X and Bluesky. Numbered for posting in order. Replace `<repo>` with the canonical link before posting.*

---

**1/7**

Just shipped v0.2 of evidence — a local-first AI research assistant for high-stakes work (pharma, legal, medical, finance).

Every sentence is hyperlinked to the exact source span. If the model can't cite, it refuses to answer.

Three rules make this work. Thread 👇

---

**2/7**

**Rule 1: existence.**

Default RAG has the model emit `[12]` as a citation and trusts it to count. Models can't count.

evidence makes citations integers — primary-key references into a span table. If the span doesn't exist, the answer is refused before it reaches the user.

---

**3/7**

**Rule 2: in-context.**

Subtler. A model can cite a real span — just one it wasn't given for this question. The footnote looks legitimate. The model never read it.

evidence keeps a HashSet of every span shown and rejects any citation outside it. This is the rule that makes the output auditable.

---

**4/7**

**Rule 3: support.**

Hardest to enforce. Right now I use a cross-encoder as a proxy for entailment — score the (sentence, cited-span) pair, accept if a threshold clears.

This catches "off-topic" citations. It doesn't catch contradictions yet. That's v0.3's NLI swap.

---

**5/7**

The design choice I'm proud of: SupportChecker is a trait. v0.2 ships RerankerSupportChecker. v0.3 swaps in a real NLI model. Every consumer of the trait gets the upgrade for free.

Ship the contract before the perfect implementation. Iterate behind a stable seam.

---

**6/7**

evidence is one binary, an SQLite file, and a local LLM. No vendor account, no API key, no telemetry.

Built for the kind of work where plausible-but-wrong gets quoted as fact: FDA submissions, depositions, signed quarterlies.

The refusal is the feature, not a bug.

---

**7/7**

Repo, README, design doc, and the v0.2 release: <repo>

If any of this resonates and you'd find it useful, issues / PRs / brutal feedback all welcome.

Next up (v0.3): Tauri desktop UI with a PDF viewer where citation chips actually click through.
