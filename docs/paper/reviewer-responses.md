# Reviewer responses — rebuttal preparation

> Internal working document. For each anticipated reviewer attack: the
> attack as a reviewer would phrase it, a severity rating, and the
> strongest *honest* response grounded in the now-measured numbers and
> the artifact that proves each one. Numbers trace verbatim to the
> source docs (`CLAIMS.md`, `benchmark-results.md`, `rule-metrics.md`,
> `nli-comparison.md`, `cost-latency.md`, `faithfulness-audit.md`,
> `fda_label_bench_PROVENANCE.md`, `section5-results.draft.md`,
> `paper-draft.md`). The honesty section at the end states the points
> we cannot fully defend yet; we do not paper over them.

Severity legend: **Critical** (could sink the paper if unanswered) ·
**High** (a skeptical reviewer will press hard) · **Medium** (expected,
answerable) · **Low** (easily dispatched).

---

## Summary table

| # | Attack | Severity | One-line response |
|---|---|---|---|
| 1 | "The decomposition is obvious in retrospect." | High | Obvious ≠ measured; the non-overlap matrix is the contribution, and it had to be run to know it. |
| 2 | "100% non-overlap is baked in by nested policies." | Critical | It is a measured *blindness* property — OOC 254/254 invisible to existence-only, support 508/508 invisible to two-gate — not policy nesting. |
| 3 | "Your injected failures are too easy." | High | Conceded; matched-pair design partially defends; the natural-failure run (§5.6) is honestly open. |
| 4 | "It's an AI-authored benchmark — untrustworthy." | Critical | Datasheet + automated audit: 249 exact / 1 fuzzy / 5 flagged-all-verified-faithful / zero fabrication; v1→v2 self-caught bug; structural results authoring-independent. |
| 5 | "Support F1 is below 0.95 — it over-refuses valid claims." | High | Recall 0.992 (no missed failures); precision is the separately-argued §5.1 NLI conservatism; CI reported; refuse-or-resolve framing. |
| 6 | "Single domain, N too small." | Medium | Conceded explicitly; workshop scope; CIs reported; no generalization claimed. |
| 7 | "Self-RAG / GopherCite already do this." | High | Neither externalizes the three gates nor measures non-overlap; Self-RAG has no in-prompt check; GopherCite enforces, doesn't validate. |
| 8 | "This is just ALCE." | High | ALCE makes existence & in-context true *by construction*; we measure exactly what ALCE assumes away. |
| 9 | "No latency / cost numbers." | Medium | Now measured (§5.7): structural 2.5% vs NLI 97.5% of wall time. |
| 10 | "Correctness ≠ causal faithfulness." | Medium | Explicitly scoped out; we claim correctness at the validator boundary, never causal reliance. |
| 11 | "F1/lift numbers come from a class-balanced set." | Medium | Conceded in the paper; false-refusal rate reported explicitly, not optimized away. |
| 12 | "Not a new model — what's the contribution?" | Medium | Taxonomic + empirical; the decomposition + the measured evidence it holds. |
| 13 | "NLI for faithfulness isn't new." | Low | We never claim it is; NLI is one rule in a three-rule decomposition. |

---

## 1. "The decomposition is obvious in retrospect."

**Attack (reviewer voice).** "Splitting citation correctness into
existence / in-context / support is intuitive — anyone designing a RAG
validator would arrive at this. Where is the contribution beyond a
taxonomy?"

**Severity.** High. This is the framing-kill attack: if the taxonomy
reads as self-evident, the paper looks thin.

**Response.** Obvious-in-retrospect is not the same as
measured-to-be-distinct. The contribution is not the three names; it is
the empirical demonstration that the three catch *non-overlapping*
error classes, plus the composition curve. Before the experiment, it
was a live possibility that the support gate (rule 3) would also catch
out-of-context errors (an out-of-context span is often also
unsupported), collapsing rules 2 and 3 into one signal — exactly the
"window dressing" failure mode anticipated in `CLAIMS.md` ("If rules 2
and 3 catch the same errors at >50% overlap, the decomposition is
window dressing"). The measured result rules this out: out-of-context
citations are accepted by existence-only **254/254 (100%)** and support
failures are accepted by two-gate **508/508 (100%)**
(`rule-metrics.md` §5.3). The decomposition is justified *because* the
data forces it, not because the names are tidy. We lead the paper with
the non-overlap matrix, not the taxonomy, precisely to make this point
(`section5-results.draft.md` §5.3, "the kernel").

---

## 2. "100% non-overlap is by construction of the nested policies."

**Attack (reviewer voice).** "You report 100% non-overlap, but your
policies are nested — `vanilla` ⊂ `existence_only` ⊂ `two_gate` ⊂
`three_gate`. Of course a later gate 'catches' what an earlier policy
didn't refuse; you've defined the gates so each failure class is only
reachable at one gate. The 100% is an artifact of the construction, not
a finding."

**Severity.** Critical. This is the single most dangerous attack — if
it lands, the kernel result is meaningless. It must be answered
crisply.

**Response.** The 100% is a *measured blindness property* of the data,
not a consequence of policy layering. The claim is not "a later gate
refuses what an earlier policy passed" (which would indeed be near-
tautological for a nested stack). The claim is that each earlier rule
is *constitutively blind* to the later class by virtue of *what it
checks*, independent of stacking order:

- **Out-of-context errors are invisible to the existence rule because
  the cited span genuinely exists.** The existence check is a
  primary-key lookup; an out-of-context citation points at a real
  corpus span (it was duplicated to a fresh ID *outside* the prompt by
  the matched-pair in-context operator — see `paper-draft.md` §4.4).
  The existence gate returns "present" and *cannot* see the failure no
  matter where it sits in the stack. Measured: OOC accepted by
  existence-only **254/254 (100%)** (`rule-metrics.md` §5.3).
- **Support failures are invisible to both structural rules because the
  span exists and is in context.** A support mutation perturbs the
  *claim/span entailment* on the same cited, in-prompt span; existence
  and in-context both pass by construction of the failure, not of the
  policy. Measured: unsupported+contradicted accepted by two-gate
  **508/508 (100%)** (`rule-metrics.md` §5.3).

The disjointness is therefore a property of *what each rule can
physically observe given the failure*, which would hold even if the
gates were evaluated independently rather than nested. The paper states
this explicitly: "This is a *measured blindness property*, not an
artifact of the nested-policy construction… each gate is blind to the
others' classes by what it checks, not by how the policies are layered"
(`section5-results.draft.md` §5.3; `benchmark-results.md` "Reading
it"). The nested-stack design is a separate, *conservative* choice for
the lift number, addressed in attack #12-adjacent reasoning below: only
`vanilla` and `existence_only` are genuinely single-rule, so the
+56.3-point lift is computed against the *strongest available* single
rule, making it a lower bound (`rule-metrics.md` Claim 3;
`section5-results.draft.md` §5.4).

---

## 3. "Your injected failures are too easy / synthetic."

**Attack (reviewer voice).** "Failures are produced by deterministic
injection operators. Real LLM citation errors are subtler. High catch
rates on synthetic perturbations don't tell us the validator works in
the wild."

**Severity.** High. Legitimate; we concede the core of it.

**Response.** We concede the limitation rather than argue around it,
and it is surfaced proactively in `CLAIMS.md` ("Where we are likely to
be attacked", item 2), the paper's Limitations (§6, "Injection by
construction"), and the §5 threats-to-validity block. Three honest
points of partial defense:

1. **The injection is matched-pair, not adversarial-easy.** The
   in-context operator *duplicates the span's exact text at a fresh ID
   outside the prompt* and repoints the citation — so the only variable
   that changes is in-context status; the relevance/topicality confound
   is held fixed (`paper-draft.md` §4.4). This is a true matched pair
   (contrast-set methodology, Gardner et al. 2020), which makes the
   in-context result a clean attribution, not an easy one.
2. **The structural results don't depend on failure subtlety at all.**
   Existence and in-context are exact, model-independent index/string
   checks (F1 = 1.000 each, `rule-metrics.md` §5.2). "Too easy" is an
   argument about the *support* gate's realism, and there we already
   report the hard number (recall 0.992 / precision 0.873), not an
   inflated one.
3. **The intended defense is named and honestly marked open.** The
   natural-failure supplement (CLAIMS.md claim 4; `paper-draft.md`
   §4.5; `section5-results.draft.md` §5.6) — real LLM hallucinations,
   no injection, human-graded into the taxonomy — is explicitly
   `[OPEN — not run]`. We do not claim it as done. The paper's scope
   sentences ("not real-world failure prevalence") appear on every
   headline claim so a reviewer is never misled into reading the
   injected numbers as deployment numbers.

We would rather a reviewer see us state this limit plainly than catch
us hiding it.

---

## 4. "It's an AI-authored benchmark — you can't trust the seeds."

**Attack (reviewer voice).** "The `valid` seeds were generated by an
LLM, not written by humans. The benchmark could contain fabricated
spans, paraphrases passed off as verbatim citations, or systematic
authoring bias that flatters the validator."

**Severity.** Critical. For a benchmark paper this is the existential
attack and the reviewer will be right to push.

**Response.** Disclosed in full and defended with an automated audit
plus a self-caught bug:

1. **Full datasheet.** Authoring method, constraints, and limitations
   are documented in `fda_label_bench_PROVENANCE.md` — including the
   explicit statement "The `valid` seed examples were AI-authored, not
   human-written." We do not bury the method; the datasheet is cited
   from §4.2 and §5.0 of the paper.
2. **Automated faithfulness audit, manually resolved.**
   `evidence-eval audit-faithfulness` checks every cited span verbatim
   against the source corpus. Over 254 examples / 255 spans:
   **249 exact, 1 fuzzy, 5 flagged (missing)**
   (`faithfulness-audit.md` totals). Every one of the 6 flags was then
   manually located in its source PDF and confirmed to be **genuine
   verbatim label text** — flagged only because the audit tool does not
   collapse interior whitespace or merge bullet-glyph / line-split /
   hyphen-break artifacts (`faithfulness-audit.md`, "Manual
   verification of flagged spans"; `fda_label_bench_PROVENANCE.md`,
   "Automated faithfulness audit"). Net: **all 255 cited spans faithful
   to source; zero fabricated or unverifiable spans.** No example was
   silently removed (`cmc_eyedrops_storage` was confirmed against
   source and kept, text already correct).
3. **The v1→v2 self-caught bug is evidence, not embarrassment.** A v1
   benchmark (263 seeds) showed 49% false-refusal on valid examples. A
   hand audit against source PDFs traced this to an *authoring*
   artifact — line-level span extraction + a "cite minimally" guideline
   produced fragment citations (e.g. cited `"Doxycycline is virtually
   completely"` with *absorbed* in the next, uncited line). We fixed
   the guideline (one complete self-contained sentence), re-authored
   all seeds (v2), and the 49% decomposed into **~15 points fixed bug +
   ~29% genuine NLI conservatism** (`benchmark-results.md`, "The v1
   bug"; `fda_label_bench_PROVENANCE.md`, "Audit history"). Catching
   our own benchmark bug by audit *before* shipping is part of why the
   remaining numbers are credible.
4. **The structural results are authoring-independent.** Existence and
   in-context (F1 = 1.000) and the entire non-overlap result do not
   depend on span text — they are index/string checks
   (`section5-results.draft.md` §5.3 threats; `benchmark-results.md`,
   "Rules 1 & 2 are flawless"). Only the support-gate numbers carry an
   authoring-homogeneity caveat, and that caveat is stated in §6 and
   the datasheet rather than left for the reviewer to find.

---

## 5. "Support F1 is below 0.95 — the validator over-refuses valid claims."

**Attack (reviewer voice).** "Your own target was F1 ≥ 0.95 per rule.
Support F1 is 0.929, precision 0.873 — the validator wrongly refuses
~13% of valid claims it sees. A validator that cries wolf is not
deployable."

**Severity.** High. The one soft number in the paper; a reviewer will
zero in.

**Response.** We do not hide it; we measure it as a separate column and
argue it deliberately:

- **Recall is 0.992 — the gate does not miss bad citations.** Of 508
  genuine support failures, it catches 504 (tp/fp/fn = 504/73/4,
  `rule-metrics.md` §5.2). The failure mode is *false-refuse-on-valid*,
  never *missed-failure* (`benchmark-results.md`, "The support gate
  works on real failures"). For pharmaceutical text, where the cost of
  a confidently-wrong citation is high, a high-recall conservative gate
  is the correct default — this is the explicit **refuse-or-resolve**
  deployment philosophy (`paper-draft.md` §3.5).
- **The precision gap *is* the §5.1 finding, not a missed failure.**
  The 73 false positives are valid claims the NLI checkpoint
  conservatively declines. This persists even with complete-sentence
  citations and a strong DeBERTa-v3 MNLI model (~29% of true claims
  still refused; `benchmark-results.md`, "Reading it";
  `section5-results.draft.md` §5.1). We present it as a measured
  property of NLI-as-support-gate, which is itself a result worth
  reporting, not a flaw the paper failed to fix.
- **It is reported with a confidence interval.** Support F1 = 0.929
  **[0.91, 0.94]**, 95% bootstrap CI (B=1000, fixed seed, resample over
  1,270 examples; `rule-metrics.md` §5.2). We do not present the point
  estimate bare.
- **The decomposition's value does not rest on this number.** Two of
  three rules exceed the target at F1 = 1.000; the headline non-overlap
  and structural-exactness results are model-independent
  (`CLAIMS.md`, "The empirical claim, more precisely", item 1). The
  paper's claim is the *decomposition*, and the support gate's
  conservatism is presented as the open frontier (§7), not swept under
  an aggregate score.

---

## 6. "Single domain, N is too small."

**Attack (reviewer voice).** "254 seeds from one domain (FDA labels).
This doesn't support general claims about RAG citation faithfulness."

**Severity.** Medium. Expected; fully conceded.

**Response.** Conceded explicitly and repeatedly — this is a workshop-
scope artifact and we say so. `CLAIMS.md` ("Domain and scope"; "What
the paper does NOT claim": "Not generalization beyond the benchmark
domain") and `paper-draft.md` §6 ("Domain. FDA labels only. No
cross-domain generalization claim." / "Size. 254 seeds / 1,270
examples — workshop-appropriate, not main-conference scale."). Every
headline claim is scoped in-line to "the v2 injected FDA-label
benchmark (not real-world failure prevalence)". We report 95% bootstrap
CIs on the soft numbers (support F1 [0.91, 0.94], three-gate F1
[0.96, 0.97]; `rule-metrics.md`) rather than pretend the sample is
larger than it is. FDA labels were chosen because they are
public-domain, structured, and legally meaningful (`CLAIMS.md`, "Why
FDA labels"); cross-domain and human-curated expansion is named as
future work (§7). Narrow honest claims are the intended posture, not an
oversight.

---

## 7. "Self-RAG / GopherCite already do this."

**Attack (reviewer voice).** "Self-RAG has `IsRel` and `IsSup`
reflection tokens; GopherCite enforces grounded quotes. The
decomposition is not novel."

**Severity.** High. `CLAIMS.md` revision log flags Self-RAG as "the
most likely reviewer-attack vector".

**Response.** Neither subsumes the contribution, and we say precisely
why in §2:

- **Self-RAG** has reflection tokens for retrieval relevance and
  support, but they are *internalized in the model*, not externalized
  as a validator gate that refuses output, and — critically — **none
  of them ask "was this span in the prompt for *this* query."** The
  in-context dimension (the closed-loop "a model may only cite what it
  was shown" constraint, `paper-draft.md` §3.2) has no Self-RAG
  analogue. Self-RAG is the *closest* existing partial decomposition,
  and we credit it as such (`CLAIMS.md` revision log; `paper-draft.md`
  §2) rather than overstate the gap.
- **GopherCite** *enforces* existence and in-context through
  constrained decoding — it makes them true by construction at
  generation time — rather than *validating* them post-hoc as
  independent, separately measured failure modes (`paper-draft.md` §2).
  It cannot measure non-overlap because it never lets the failures
  occur.
- **Neither measures that the dimensions catch non-overlapping
  errors.** That measurement — the kernel result — is what no prior
  work does (`paper-draft.md` §2, "The gap"; full matrix in
  `related-work-table.md`). The prior-art reads were web-verified and
  the three pre-submission lock searches run (`CLAIMS.md` revision
  log, 2026-05-15 entries).

---

## 8. "This is just ALCE."

**Attack (reviewer voice).** "ALCE already measures RAG citation
quality with precision/recall. Your benchmark is a re-skin."

**Severity.** High.

**Response.** ALCE (and HAGRID, ExpertQA) restrict citations to integer
markers into the retrieved set, which makes **existence and in-context
true *by construction* and therefore unmeasurable** (`paper-draft.md`
§2, "RAG citation benchmarks"). Our work measures exactly the two
dimensions ALCE assumes away — it is an explicit form of what ALCE
"smuggles into its setup" (`CLAIMS.md` revision log, 2026-05-15). We
also do not claim to supplant ALCE: "Not a benchmark contribution per
se… It is not designed to supplant ALCE, HAGRID, ExpertQA"
(`CLAIMS.md`, "What the paper does NOT claim"). An explicit ALCE-subset
translation experiment is named as future work and honestly marked
`[OPEN — not run]` (`paper-draft.md` §5.5; `section5-results.draft.md`
§5.5) — we do not present a comparison we have not run.

---

## 9. "No latency or cost numbers."

**Attack (reviewer voice).** "You argue the decomposition has
deployment value but give no cost evidence. Without latency this is a
paper claim, not an engineering one."

**Severity.** Medium. Previously a real gap; now measured.

**Response.** Measured and reported in §5.7 (`cost-latency.md`,
reproducible via `evidence-eval latency`). Harness-level wall-clock
decomposition over 1,270 examples (5,080 timed calls):

| stage | mean / call | share |
|---|---|---|
| structural (existence + in-context + harness) | 140.5 µs | **2.5%** |
| support gate (NLI forward passes) | 5.6 ms | **97.5%** |
| total (real-NLI run) | 5.7 ms | 100% |

The two exact structural gates are effectively free (~40× cheaper than
the NLI pass); this is the deployment argument for the decomposition —
run the cheap exact gates unconditionally, apply the expensive semantic
gate only where they pass. We state the measurement honestly as
harness-level (mock-run baseline vs. real-NLI delta), order-of-
magnitude not microbenchmark, and note existence vs. in-context are not
separated (both O(µs), dwarfed by NLI) — the method note is in
`cost-latency.md` and quoted in §5.7. We do not over-claim
microbenchmark precision.

---

## 10. "Citation correctness ≠ causal faithfulness."

**Attack (reviewer voice).** "Your in-context gate shows a span was
*available* to the model, not that the model *used* it. The model can
post-rationalize a well-formed citation. You haven't shown faithful
attribution."

**Severity.** Medium. The reviewer is correct; we agree and scoped it
out in advance.

**Response.** We agree, and the paper claims correctness, never causal
faithfulness. This is scoped explicitly in three places: `CLAIMS.md`
("Scope (correctness, not causal faithfulness)… We do NOT claim causal
citation faithfulness — the in-context gate shows the span was
*available* to the model, not that the model *relied* on it; post-
rationalization is possible"), `paper-draft.md` §3 ("Scope of the
claim: citation correctness, not causal faithfulness") and the §3.0
formal definition ("in_context establishes that s was *available* to
the model, not that the model *used* it"), and §6 Limitations. Causal
reliance (counterfactual span removal, attribution tracing) is named as
a distinct, out-of-scope fourth dimension. The boundary is deliberate:
existence, in-context, and support are checkable at the validator
without model internals; causal faithfulness is not. We cite the
recent post-rationalization literature ourselves rather than wait to be
told about it (`paper-draft.md` §2, §3 Scope).

---

## 11. "Your F1/lift numbers come from a class-balanced injected set."

**Attack (reviewer voice).** "The +56-point lift and the F1 ladder are
computed on a set where failures are prevalent. In real RAG, failures
are rare and false-refusal cost dominates. These numbers overstate
deployment value."

**Severity.** Medium. Valid; conceded in the paper.

**Response.** Conceded in the paper before the reviewer raises it.
`CLAIMS.md` ("Additional limitations we surface proactively": "The
injected benchmark is class-balanced (failures prevalent); the +F1 /
lift numbers do not reflect real-world failure prevalence") and
`paper-draft.md` §6 ("Headline F1/lift numbers come from a class-
balanced injected set where failures are prevalent; real RAG has lower
failure prevalence where false-refusal cost dominates. We report the
support gate's false-refusal rate explicitly rather than optimize it
away"). The support gate's conservatism (precision 0.873, the ~29%
valid over-refusal) is reported as its own number, not hidden inside an
aggregate. Every headline carries the in-line scope qualifier "not
real-world failure prevalence." The natural-failure run (§5.6) is the
intended realism check and is honestly marked open.

---

## 12. "Not a new model — what is the actual contribution?"

**Attack (reviewer voice).** "No new architecture, no new training, no
new NLI method. What is novel enough to publish?"

**Severity.** Medium.

**Response.** The contribution is taxonomic and empirical, stated as
such up front: "Not a new architecture. We do not propose a new ML
model. The contribution is taxonomic and empirical" (`CLAIMS.md`). The
novel, non-trivial claim is the *decomposition plus the measured
evidence it holds*: three operationally independent rules that catch
**fully disjoint** error classes on the benchmark (OOC 254/254, support
508/508 invisible to preceding gates; `rule-metrics.md` §5.3), a
monotone composition ladder (F1 0.000 → 0.400 → 0.667 → 0.963,
**+56.3-point** additive lift over the strongest non-composed baseline;
`rule-metrics.md` Claim 3), and structural rules that are exact and
model-independent (F1 = 1.000, Δ=0 across two NLI models;
`nli-comparison.md`). The non-overlap was *not* knowable without
running the experiment (attack #1). We also position the decomposition
as the right granularity for RAGAS-style frameworks to adopt
(`paper-draft.md` §2).

---

## 13. "Using NLI for faithfulness is not new."

**Attack (reviewer voice).** "NLI-as-faithfulness is well-traveled
(TRUE, Honovich 2022). Nothing new here."

**Severity.** Low. We claim no novelty there.

**Response.** Agreed and pre-stated: "Not 'first to use NLI for
faithfulness.' That lineage starts with Honovich '22 (TRUE) and is
well-traveled. We are using NLI as one rule in a three-rule
decomposition, not claiming the NLI move itself" (`CLAIMS.md`). TRUE is
cited in §2 as establishing NLI as a faithfulness signal but
orthogonal to the decomposition question (`paper-draft.md` §2). The
support gate is one of three rules; the contribution lives in the
decomposition and the non-overlap measurement, not in the choice of
NLI.

---

## Weakest points we cannot fully defend yet

These are stated plainly. We would rather a reviewer see us name them
than discover them.

1. **The natural-failure run (§5.6) is not done.** This is the single
   most important open item. Every "injected failures are too easy"
   concern (attack #3) ultimately routes here, and the defense is a
   plan, not a result. CLAIMS.md claim 4 is parameterized with an
   unmeasured `[Z to be measured]`. The matched-pair design and the
   model-independent structural exactness partially defend, but they do
   not substitute for real LLM hallucinations graded into the taxonomy.
   Until §5.6 runs, the deployment-realism claim is scoped, not
   demonstrated. We mark it `[OPEN — not run]` everywhere and never
   imply otherwise.

2. **AI-authoring homogeneity on the support gate.** The structural
   results are authoring-independent, but the support-gate numbers
   (precision 0.873 in particular) carry a genuine
   authoring-homogeneity caveat: AI-authored seeds from a single
   corrected guideline are structurally more uniform than independent
   hand curation, and the entailment quality of the `valid` /
   mutation pairs was only **spot-audited (3 labels)** plus the full v1
   false-refusal audit — not an exhaustive human pass over all 254
   (`fda_label_bench_PROVENANCE.md`; `benchmark-results.md` caveats;
   `paper-draft.md` §6). A wider human entailment-verification pass is
   the conservative path and has not been done.

3. **No inter-annotator agreement.** There is no IAA number because the
   seeds were AI-authored and the audits were single-pass author
   verification against source PDFs. We have span-faithfulness
   verification (every flagged span manually confirmed) but no
   multi-rater semantic-entailment agreement statistic
   (`fda_label_bench_PROVENANCE.md`, "Known limitations";
   `section5-results.draft.md` threats). A reviewer asking for IAA is
   asking for something we do not have, and we should concede it
   directly rather than deflect.

4. **Single retrieval config, one NLI checkpoint pair.** Results are
   from one retrieval configuration and two MNLI checkpoints
   (distilbert, DeBERTa-v3); the structural results do not depend on
   either, but support-gate generality across retrieval setups and NLI
   models beyond this pair is not established
   (`benchmark-results.md` caveats; `section5-results.draft.md`
   threats).

The honest posture for the rebuttal: the structural core (existence,
in-context, non-overlap, cost split) is hard, model-independent, and
authoring-independent, and we defend it without reservation. The
support-gate conservatism is a measured finding we present rather than
hide. The natural-failure run, authoring homogeneity, and absent IAA
are real gaps; we name them, scope every claim around them, and point
to the explicit future work rather than overclaim.

---

## Attacks covered (index)

1 decomposition obvious · 2 non-overlap by construction (critical) ·
3 injected failures too easy · 4 AI-authored benchmark (critical) ·
5 support F1 below 0.95 / over-refusal · 6 single domain / small N ·
7 Self-RAG / GopherCite prior art · 8 just ALCE · 9 no latency/cost ·
10 correctness ≠ causal faithfulness · 11 class-balanced set ·
12 no new model / contribution · 13 NLI not novel · plus the
"weakest points we cannot fully defend yet" honesty section
(natural-failure run not done; AI-authoring homogeneity; no IAA;
single retrieval/NLI config).
