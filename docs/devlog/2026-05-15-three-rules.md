# Three rules that make AI citations trustworthy

*2026-05-15 · evidence v0.2.0 shipped*

I just tagged `v0.2.0` of [evidence](https://github.com/Bilalsabry/evidence), a local-first AI research assistant for high-stakes work — pharma, legal, medical, finance. The whole point of the project is one feature: every sentence in the answer is hyperlinked to a specific span of text on a specific page of a specific document. If the model can't cite, the answer is refused.

This post is about why "with citations" isn't enough. Three rules turn out to matter, and getting one wrong defeats the others.

## Rule 1: existence

> Every cited span must exist in the database.

Sounds trivial. It isn't. The default RAG implementation has the model emit a free-text citation like `[12]` or `(source: trial.pdf p7)` and then trusts the model to count rows in its head. Models can't count. They make up `[12]` when there are only ten sources. They cite "page 7" when the document has six pages.

In `evidence`, citations aren't free text. The model is shown a structured context — `chunk_id=42, span_ids=120..135, text=…` — and required to return a structured response where every `span_id` is an integer. The validator looks up the span by primary key. If it's not there, the answer is refused before it reaches the user.

This is the cheapest rule to enforce and the one most production RAG systems skip.

## Rule 2: in-context

> Every cited span must be in the chunks the model was shown.

This is the subtler one. Models trained on the open web have *seen* a lot of medical-trial spans. A clever model will cite a span that really exists in the corpus — just one it wasn't given for this question. The footnote looks legitimate. The model never read it. The reader has no way to know.

`evidence` walks the response and rejects any `span_id` that wasn't in the prompt. The validator keeps a `HashSet<i64>` of every span shown to the model and checks every citation against it. Out-of-context citations fail loudly:

```
model cited span 999, which was not in the chunks shown
```

This rule, more than anything else, is what turns "RAG with footnotes" into auditable AI. It's the difference between "the system thinks this is supported" and "the system can prove the model saw the supporting text."

## Rule 3: support

> The cited spans must lexically support the claim.

This is the rule that's hardest to get right, because at this point you're asking a model to grade another model's work.

In v0.1, I shipped the first two rules and explicitly skipped this one — the design doc called it out as a v0.2 deliverable. In v0.2, I wired up `RerankerSupportChecker`: a cross-encoder scores each `(sentence, cited_span_text)` pair, and the sentence is only accepted if some cited span clears a threshold. The cross-encoder is doing entailment by proxy — it was trained for relevance, not natural-language inference — but it's a real signal, and the trait is shaped so a proper NLI model is a drop-in swap when one lands in v0.3.

The "claim" of `Supports` is currently weak. A high cross-encoder score means "this text is relevant to that sentence," not "this text *entails* that sentence." A model could still write *"the trial enrolled 1000 patients"* with a citation to *"the trial enrolled 240 patients"* and slip past — both texts are about trial enrollment, the cross-encoder happily scores them as related.

The fix is the NLI swap, scheduled for v0.3. The interesting design choice is that v0.2 ships the contract even though the model is wrong: `SupportChecker` is a trait, `RerankerSupportChecker` is one implementation, and every consumer of the trait gets the upgrade for free once the right model is wired in.

## What this buys

A small, deliberate constraint set:

- The model can write any sentence it wants — as long as it can point at the source.
- The user can audit every claim — by clicking through to the highlighted span.
- The system fails closed — the refusal is the feature, not a bug.

`evidence` is one binary, an SQLite file, and a local LLM. No vendor account, no API key, no telemetry. It's the kind of tool I'd want if I were reviewing a draft FDA submission, or a deposition exhibit, or a quarterly that's about to be signed. The whole point is to make the AI part of the workflow falsifiable.

## What's next

v0.3 is the desktop UI: a Tauri shell with a real PDF viewer where citation chips actually click through to a highlighted rectangle on the right page. And the NLI swap, finally. Issues are filed [here](https://github.com/Bilalsabry/evidence/milestone/3).

If any of this resonates and you'd find it useful, the repo is at [Bilalsabry/evidence](https://github.com/Bilalsabry/evidence). Issues, PRs, and brutal feedback all welcome.
