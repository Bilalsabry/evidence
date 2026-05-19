//! Renders a markdown report of an eval run.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::dataset::HallucinationClass;
use crate::runner::{Outcome, Policy, RefusalReason, RunResult};

/// Aggregated per-class, per-policy view: how many examples of each class
/// were caught (refused) by each policy. The paper's claim shows up here:
/// each gate's catch rate per class should match the [`expected_outcome`]
/// matrix.
pub struct Report {
    pub rows: Vec<RunResult>,
}

impl Report {
    #[must_use]
    pub fn new(rows: Vec<RunResult>) -> Self {
        Self { rows }
    }

    /// Cell-by-cell accept/refuse count for each (class, policy).
    /// Returned in deterministic order for stable diffing.
    #[must_use]
    pub fn matrix(&self) -> BTreeMap<(HallucinationClass, Policy), (usize, usize)> {
        let mut m: BTreeMap<(HallucinationClass, Policy), (usize, usize)> = BTreeMap::new();
        for row in &self.rows {
            let entry = m.entry((row.class, row.policy)).or_insert((0, 0));
            match &row.outcome {
                Outcome::Accepted => entry.0 += 1,
                Outcome::Refused(_) => entry.1 += 1,
            }
        }
        m
    }

    /// Number of rows where the validator behaved as the class predicted.
    #[must_use]
    pub fn agreement(&self) -> (usize, usize) {
        let agree = self.rows.iter().filter(|r| r.agreement).count();
        (agree, self.rows.len())
    }
}

/// Per-class (agree, total) over the `ThreeGate` policy only — the one
/// policy whose verdict depends on the NLI model. Deterministic order.
fn three_gate_class_agreement(rows: &[RunResult]) -> BTreeMap<HallucinationClass, (usize, usize)> {
    let mut m: BTreeMap<HallucinationClass, (usize, usize)> = BTreeMap::new();
    for row in rows.iter().filter(|r| r.policy == Policy::ThreeGate) {
        let e = m.entry(row.class).or_insert((0, 0));
        e.1 += 1;
        if row.agreement {
            e.0 += 1;
        }
    }
    m
}

fn class_label(c: HallucinationClass) -> &'static str {
    use HallucinationClass as C;
    match c {
        C::Valid => "valid",
        C::Uncited => "uncited",
        C::FabricatedSpan => "fabricated_span",
        C::OutOfContext => "out_of_context",
        C::Unsupported => "unsupported",
        C::Contradicted => "contradicted",
    }
}

/// Precision / recall / F1 for one marginal gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prf {
    pub tp: usize,
    pub fp: usize,
    pub fn_: usize,
}

impl Prf {
    #[must_use]
    pub fn precision(&self) -> f64 {
        let d = self.tp + self.fp;
        if d == 0 {
            1.0
        } else {
            self.tp as f64 / d as f64
        }
    }
    #[must_use]
    pub fn recall(&self) -> f64 {
        let d = self.tp + self.fn_;
        if d == 0 {
            1.0
        } else {
            self.tp as f64 / d as f64
        }
    }
    #[must_use]
    pub fn f1(&self) -> f64 {
        let (p, r) = (self.precision(), self.recall());
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

/// Tiny deterministic PRNG (SplitMix64). Inlined on purpose: the
/// bootstrap must be seedable and reproducible without pulling a `rand`
/// dependency into the eval crate. SplitMix64 is the standard seeder
/// recommended alongside xoshiro — fast, full-period, good enough for
/// resampling indices.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `[0, n)`. `n` must be non-zero. Uses Lemire's
    /// debiased multiply-shift so the resample is not modulo-skewed.
    fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0, "below(0) is undefined");
        let n = n as u64;
        loop {
            let x = self.next_u64();
            let m = u128::from(x) * u128::from(n);
            let lo = m as u64;
            if lo >= n {
                return (m >> 64) as usize;
            }
            // Rejection zone: re-roll until the low word clears the
            // threshold so every bucket is equally likely.
            let threshold = n.wrapping_neg() % n;
            if lo >= threshold {
                return (m >> 64) as usize;
            }
        }
    }
}

/// 2.5 / 97.5 percentile interval of a sample, via the
/// nearest-rank method on the sorted copy. Returns `(lo, hi)`.
/// Empty input → `(0.0, 0.0)`.
fn percentile_ci(samples: &[f64]) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut s = samples.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (percentile(&s, 2.5), percentile(&s, 97.5))
}

/// Nearest-rank percentile of an already-sorted slice. `p` in [0, 100].
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    // Nearest-rank: rank = ceil(p/100 * n), clamped to [1, n].
    let rank = (p / 100.0 * n as f64).ceil() as usize;
    let idx = rank.clamp(1, n) - 1;
    sorted[idx]
}

/// One bootstrap resample of `0..n` index positions, drawn with
/// replacement. Deterministic given the PRNG state.
fn resample_indices(rng: &mut SplitMix64, n: usize) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    (0..n).map(|_| rng.below(n)).collect()
}

/// Number of bootstrap resamples for the §5 confidence intervals.
const BOOTSTRAP_B: usize = 1000;
/// Fixed seed so the CIs are reproducible across runs of the same data.
const BOOTSTRAP_SEED: u64 = 0x5150_4143_4954_4900;

/// Format a point estimate with its bootstrap 95% interval, e.g.
/// `0.929 [0.91, 0.95]`.
fn fmt_ci(point: f64, ci: (f64, f64)) -> String {
    format!("{point:.3} [{:.2}, {:.2}]", ci.0, ci.1)
}

/// Per-example refusal state across the four nested policies.
#[derive(Clone)]
struct ExampleRow {
    class: HallucinationClass,
    refused: std::collections::HashMap<Policy, bool>,
}

fn group_examples(rows: &[RunResult]) -> Vec<ExampleRow> {
    let mut m: BTreeMap<String, ExampleRow> = BTreeMap::new();
    for r in rows {
        let e = m.entry(r.example.clone()).or_insert_with(|| ExampleRow {
            class: r.class,
            refused: std::collections::HashMap::new(),
        });
        e.refused
            .insert(r.policy, matches!(r.outcome, Outcome::Refused(_)));
    }
    m.into_values().collect()
}

fn refused(e: &ExampleRow, p: Policy) -> bool {
    e.refused.get(&p).copied().unwrap_or(false)
}

/// Treat a policy as a binary "should this citation be refused?"
/// classifier over the whole injected set: positive = the example is
/// an injected failure (class != Valid) and so *should* be refused;
/// prediction = the policy refused it. Used for the claim-3 ablation
/// ladder.
fn policy_refusal_prf(ex: &[ExampleRow], p: Policy) -> Prf {
    let mut m = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };
    for e in ex {
        let is_failure = e.class != HallucinationClass::Valid;
        let pred = refused(e, p);
        match (is_failure, pred) {
            (true, true) => m.tp += 1,
            (true, false) => m.fn_ += 1,
            (false, true) => m.fp += 1,
            (false, false) => {}
        }
    }
    m
}

/// Marginal-gate PRF for the three rules over an example set. Pulled
/// out of `rule_metrics` so the bootstrap can recompute it on each
/// resample. Returns (existence, in-context, support).
fn rule_prfs(ex: &[ExampleRow]) -> (Prf, Prf, Prf) {
    use HallucinationClass as C;
    let mut existence = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };
    let mut incontext = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };
    let mut support = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };
    for e in ex {
        let r_exist = refused(e, Policy::ExistenceOnly);
        let r_two = refused(e, Policy::TwoGate);
        let r_three = refused(e, Policy::ThreeGate);
        match (e.class == C::FabricatedSpan, r_exist) {
            (true, true) => existence.tp += 1,
            (true, false) => existence.fn_ += 1,
            (false, true) => existence.fp += 1,
            (false, false) => {}
        }
        if !r_exist {
            match (e.class == C::OutOfContext, r_two) {
                (true, true) => incontext.tp += 1,
                (true, false) => incontext.fn_ += 1,
                (false, true) => incontext.fp += 1,
                (false, false) => {}
            }
        }
        if !r_two {
            let owned = matches!(e.class, C::Unsupported | C::Contradicted);
            match (owned, r_three) {
                (true, true) => support.tp += 1,
                (true, false) => support.fn_ += 1,
                (false, true) => support.fp += 1,
                (false, false) => {}
            }
        }
    }
    (existence, incontext, support)
}

/// The four F1s the §5 bootstrap reports a CI for: per-rule
/// (existence, in-context, support) marginal F1 plus the three-gate
/// should-refuse F1 over the whole set.
fn bootstrap_f1s(ex: &[ExampleRow]) -> [f64; 4] {
    let (e, i, s) = rule_prfs(ex);
    let three_gate = policy_refusal_prf(ex, Policy::ThreeGate).f1();
    [e.f1(), i.f1(), s.f1(), three_gate]
}

/// 95% bootstrap CIs for the four §5 F1 statistics. Resamples the
/// per-example list WITH REPLACEMENT `BOOTSTRAP_B` times under a fixed
/// seed, recomputes all four F1s on each resample, and returns the
/// 2.5/97.5 percentile interval for each. Deterministic.
fn bootstrap_f1_cis(ex: &[ExampleRow]) -> [(f64, f64); 4] {
    let mut rng = SplitMix64::new(BOOTSTRAP_SEED);
    let mut draws: [Vec<f64>; 4] = [
        Vec::with_capacity(BOOTSTRAP_B),
        Vec::with_capacity(BOOTSTRAP_B),
        Vec::with_capacity(BOOTSTRAP_B),
        Vec::with_capacity(BOOTSTRAP_B),
    ];
    for _ in 0..BOOTSTRAP_B {
        let idx = resample_indices(&mut rng, ex.len());
        let sample: Vec<ExampleRow> = idx.iter().map(|&i| ex[i].clone()).collect();
        let f1s = bootstrap_f1s(&sample);
        for (k, v) in f1s.iter().enumerate() {
            draws[k].push(*v);
        }
    }
    [
        percentile_ci(&draws[0]),
        percentile_ci(&draws[1]),
        percentile_ci(&draws[2]),
        percentile_ci(&draws[3]),
    ]
}

/// §5.2 + §5.3 metrics: per-rule marginal precision/recall/F1, and the
/// non-overlap evidence (each failure class is invisible to the rules
/// that precede its owning gate).
///
/// The policies are nested (vanilla ⊂ existence ⊂ two ⊂ three), so a
/// rule is measured as its **marginal gate** — the examples refused by
/// the policy that adds it, among those the previous policy accepted.
/// Owned classes: existence → FabricatedSpan; in-context →
/// OutOfContext; support → Unsupported + Contradicted. (`inject`
/// produces no Uncited variants, so the citation-shape precondition
/// does not appear here.)
///
/// Pure over the run rows — unit-testable without a model. Structural
/// rules (existence, in-context) are model-independent; only the
/// support row depends on the NLI checkpoint used for the run.
#[must_use]
pub fn rule_metrics(rows: &[RunResult]) -> String {
    use HallucinationClass as C;
    let ex = group_examples(rows);

    // --- §5.2 marginal PRF -------------------------------------------------
    // Existence: owned = FabricatedSpan. In-context: among
    // ExistenceOnly-accepted, owned = OOC. Support: among
    // TwoGate-accepted, owned = Unsupported + Contradicted.
    let (existence, incontext, support) = rule_prfs(&ex);

    // 95% bootstrap CIs (B=1000, fixed seed) for the three per-rule F1s
    // and the three-gate should-refuse F1. Point estimates above are
    // unchanged; these only annotate them.
    let f1_cis = bootstrap_f1_cis(&ex);
    let [existence_ci, incontext_ci, support_ci, three_gate_ci] = f1_cis;

    // --- §5.3 non-overlap --------------------------------------------------
    // OutOfContext invisible to the existence rule.
    let ooc_total = ex.iter().filter(|e| e.class == C::OutOfContext).count();
    let ooc_blind = ex
        .iter()
        .filter(|e| e.class == C::OutOfContext && !refused(e, Policy::ExistenceOnly))
        .count();
    // Support failures invisible to rules 1 & 2 (two-gate).
    let sup_total = ex
        .iter()
        .filter(|e| matches!(e.class, C::Unsupported | C::Contradicted))
        .count();
    let sup_blind = ex
        .iter()
        .filter(|e| {
            matches!(e.class, C::Unsupported | C::Contradicted) && !refused(e, Policy::TwoGate)
        })
        .count();
    let pct = |n: usize, d: usize| {
        if d == 0 {
            100.0
        } else {
            n as f64 * 100.0 / d as f64
        }
    };

    let mut out = String::new();
    let _ = writeln!(out, "# Rule isolation & non-overlap (§5.2 / §5.3)\n");
    let _ = writeln!(
        out,
        "Marginal-gate decomposition over {} examples. Structural rules \
         are model-independent; the support row reflects the NLI \
         checkpoint of this run.\n",
        ex.len()
    );

    let _ = writeln!(out, "## §5.2 — per-rule isolation");
    let _ = writeln!(
        out,
        "F1 columns carry a 95% bootstrap CI (B={BOOTSTRAP_B}, \
         resample-with-replacement over the {} examples, fixed seed).\n",
        ex.len()
    );
    let _ = writeln!(
        out,
        "| rule | owned class | precision | recall | F1 (95% CI) | tp/fp/fn |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    for (name, owned, m, ci) in [
        ("existence", "fabricated_span", existence, existence_ci),
        ("in-context", "out_of_context", incontext, incontext_ci),
        ("support", "unsupported+contradicted", support, support_ci),
    ] {
        let _ = writeln!(
            out,
            "| {name} | {owned} | {:.3} | {:.3} | {} | {}/{}/{} |",
            m.precision(),
            m.recall(),
            fmt_ci(m.f1(), ci),
            m.tp,
            m.fp,
            m.fn_,
        );
    }

    let _ = writeln!(out, "\n## §5.3 — non-overlap (disjoint error classes)\n");
    let _ = writeln!(
        out,
        "- OutOfContext accepted by existence-only: **{}/{} ({:.1}%)** — \
         the existence rule is blind to in-context errors.",
        ooc_blind,
        ooc_total,
        pct(ooc_blind, ooc_total),
    );
    let _ = writeln!(
        out,
        "- Unsupported+Contradicted accepted by two-gate: **{}/{} ({:.1}%)** \
         — rules 1 & 2 are blind to support errors.",
        sup_blind,
        sup_total,
        pct(sup_blind, sup_total),
    );
    let _ = writeln!(
        out,
        "\nEach failure class is caught only at the gate that owns it; \
         the preceding rules do not see it. Higher percentages = cleaner \
         disjointness."
    );

    // --- claim 3: additive lift (ablation ladder) --------------------------
    // Each policy as a binary should-refuse classifier over the
    // injected set. The policies are nested, so the only NON-composed
    // baselines (a single rule, not a composition) are vanilla and
    // existence-only; in-context-alone and support-alone are not
    // isolable as policies. Lift = F1(three-gate) − best non-composed.
    let ladder = [
        ("vanilla_rag", Policy::VanillaRag, false),
        ("existence_only", Policy::ExistenceOnly, false),
        ("two_gate (1+2)", Policy::TwoGate, true),
        ("three_gate (1+2+3)", Policy::ThreeGate, true),
    ];
    let f1_of = |p: Policy| policy_refusal_prf(&ex, p).f1();
    let baseline = f1_of(Policy::VanillaRag).max(f1_of(Policy::ExistenceOnly));
    let composed = f1_of(Policy::ThreeGate);
    let lift = (composed - baseline) * 100.0;

    let _ = writeln!(out, "\n## Claim 3 — additive lift (ablation)\n");
    let _ = writeln!(
        out,
        "Each policy as a binary should-refuse classifier over all {} \
         examples (positive = injected failure).\n",
        ex.len()
    );
    let _ = writeln!(
        out,
        "The `three_gate` F1 carries the same 95% bootstrap CI \
         (B={BOOTSTRAP_B}, fixed seed) as §5.2.\n"
    );
    let _ = writeln!(out, "| policy | precision | recall | F1 | composed? |");
    let _ = writeln!(out, "|---|---|---|---|---|");
    for (name, p, composed_flag) in ladder {
        let m = policy_refusal_prf(&ex, p);
        let f1_cell = if p == Policy::ThreeGate {
            fmt_ci(m.f1(), three_gate_ci)
        } else {
            format!("{:.3}", m.f1())
        };
        let _ = writeln!(
            out,
            "| {name} | {:.3} | {:.3} | {f1_cell} | {} |",
            m.precision(),
            m.recall(),
            if composed_flag { "yes" } else { "no" },
        );
    }
    let _ = writeln!(
        out,
        "\n**Additive lift: +{lift:.1} F1 points** — composed validator \
         (three-gate, F1 {}) over the strongest non-composed \
         baseline (F1 {baseline:.3}). Only `vanilla_rag` and \
         `existence_only` are non-composed (single-rule) policies; \
         in-context-alone and support-alone are not isolable in a nested \
         policy stack. Target was ≥10 points.",
        fmt_ci(composed, three_gate_ci)
    );
    out
}

/// Render the §5.1 model-comparison table: per-class three-gate
/// agreement under a baseline NLI model vs. a candidate, plus the
/// delta. The `valid` row is the headline — it is the false-refusal
/// retention the DeBERTa upgrade is meant to improve. Structural
/// classes (existence / in-context) are model-independent and should
/// show Δ = 0; a non-zero Δ there is a red flag, not a result.
///
/// Pure over the two labeled run-result sets so it is unit-testable
/// without loading a model.
#[must_use]
pub fn compare_report(
    baseline_label: &str,
    baseline: &[RunResult],
    candidate_label: &str,
    candidate: &[RunResult],
) -> String {
    let b = three_gate_class_agreement(baseline);
    let c = three_gate_class_agreement(candidate);

    let mut out = String::new();
    let _ = writeln!(out, "# NLI model comparison (§5.1) — three-gate\n");
    let _ = writeln!(out, "- **baseline:** `{baseline_label}`");
    let _ = writeln!(out, "- **candidate:** `{candidate_label}`\n");
    let _ = writeln!(
        out,
        "Per-class agreement at the `three_gate` policy (the only policy \
         whose verdict depends on the NLI model). Δ is candidate − \
         baseline in agreed rows.\n"
    );
    let _ = writeln!(out, "| class | {baseline_label} | {candidate_label} | Δ |");
    let _ = writeln!(out, "|---|---|---|---|");

    // Union of classes seen in either run, deterministic order.
    let mut classes: Vec<HallucinationClass> = b.keys().chain(c.keys()).copied().collect();
    classes.sort_unstable();
    classes.dedup();

    let mut b_tot = 0usize;
    let mut b_ag = 0usize;
    let mut c_tot = 0usize;
    let mut c_ag = 0usize;
    for class in classes {
        let (ba, bt) = b.get(&class).copied().unwrap_or((0, 0));
        let (ca, ct) = c.get(&class).copied().unwrap_or((0, 0));
        b_ag += ba;
        b_tot += bt;
        c_ag += ca;
        c_tot += ct;
        let delta = i64::try_from(ca).unwrap_or(0) - i64::try_from(ba).unwrap_or(0);
        let sign = if delta > 0 { "+" } else { "" };
        let _ = writeln!(
            out,
            "| {} | {ba} / {bt} | {ca} / {ct} | {sign}{delta} |",
            class_label(class),
        );
    }
    let total_delta = i64::try_from(c_ag).unwrap_or(0) - i64::try_from(b_ag).unwrap_or(0);
    let sign = if total_delta > 0 { "+" } else { "" };
    let _ = writeln!(
        out,
        "| **all** | **{b_ag} / {b_tot}** | **{c_ag} / {c_tot}** | **{sign}{total_delta}** |\n"
    );

    let valid_b = b.get(&HallucinationClass::Valid).copied().unwrap_or((0, 0));
    let valid_c = c.get(&HallucinationClass::Valid).copied().unwrap_or((0, 0));
    let valid_delta = i64::try_from(valid_c.0).unwrap_or(0) - i64::try_from(valid_b.0).unwrap_or(0);
    let _ = writeln!(
        out,
        "**Headline (valid-retention):** {} / {} → {} / {} ({}{} false-refusals recovered). \
         This is the §5.1 payload — does the stronger model stop refusing true claims?",
        valid_b.0,
        valid_b.1,
        valid_c.0,
        valid_c.1,
        if valid_delta > 0 { "+" } else { "" },
        valid_delta,
    );

    out
}

/// Render a markdown report. Two tables: the catch matrix, then the
/// per-example detail.
#[must_use]
pub fn write_markdown(report: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Closed-Loop Citation eval — harness report\n");

    let (agree, total) = report.agreement();
    let _ = writeln!(
        out,
        "**Agreement:** {agree} / {total} rows match the expected outcome matrix.\n"
    );

    let _ = writeln!(out, "## Catch matrix\n");
    let _ = writeln!(
        out,
        "Each cell shows `accepted / refused` for the (class, policy) pair.\n"
    );
    let _ = writeln!(
        out,
        "| class \\ policy | vanilla_rag | existence_only | two_gate | three_gate |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for class in [
        HallucinationClass::Valid,
        HallucinationClass::Uncited,
        HallucinationClass::FabricatedSpan,
        HallucinationClass::OutOfContext,
        HallucinationClass::Unsupported,
        HallucinationClass::Contradicted,
    ] {
        let _ = write!(out, "| {class:?} |");
        for policy in Policy::all() {
            let (a, r) = report
                .matrix()
                .get(&(class, policy))
                .copied()
                .unwrap_or((0, 0));
            let _ = write!(out, " {a} / {r} |");
        }
        out.push('\n');
    }

    let _ = writeln!(out, "\n## Per-example detail\n");
    let _ = writeln!(
        out,
        "| example | class | policy | actual | expected | ok? |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    for row in &report.rows {
        let _ = writeln!(
            out,
            "| {} | {:?} | {} | {} | {} | {} |",
            row.example,
            row.class,
            row.policy.label(),
            describe(&row.outcome),
            describe(&row.expected),
            if row.agreement { "✓" } else { "✗" },
        );
    }
    out
}

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Accepted => "accepted".to_string(),
        Outcome::Refused(reason) => match reason {
            RefusalReason::Uncited => "refused(uncited)".to_string(),
            RefusalReason::UnknownSpan => "refused(unknown_span)".to_string(),
            RefusalReason::OutOfContext => "refused(out_of_context)".to_string(),
            RefusalReason::Unsupported => "refused(unsupported)".to_string(),
            RefusalReason::Contradicted => "refused(contradicted)".to_string(),
            RefusalReason::Other(s) => format!("refused(other: {s})"),
        },
    }
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

    #[test]
    fn splitmix64_is_deterministic_for_a_seed() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        // A different seed gives a different stream.
        let mut c = SplitMix64::new(43);
        let zs: Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        assert_ne!(xs, zs);
    }

    #[test]
    fn below_is_in_range_and_covers_buckets() {
        let mut rng = SplitMix64::new(7);
        let n = 5;
        let mut seen = [false; 5];
        for _ in 0..2000 {
            let v = rng.below(n);
            assert!(v < n, "below({n}) returned {v}");
            seen[v] = true;
        }
        // Over 2000 draws every bucket of a size-5 range should appear.
        assert!(seen.iter().all(|&s| s), "not all buckets sampled: {seen:?}");
    }

    #[test]
    fn percentile_nearest_rank_endpoints() {
        let sorted: Vec<f64> = (1..=100).map(f64::from).collect();
        // ceil(2.5/100*100)=3 -> idx 2 -> 3.0; ceil(0.975*100)=98 -> 98.0
        assert_eq!(percentile(&sorted, 2.5), 3.0);
        assert_eq!(percentile(&sorted, 97.5), 98.0);
        assert_eq!(percentile(&sorted, 100.0), 100.0);
    }

    #[test]
    fn percentile_ci_handles_single_and_empty() {
        assert_eq!(percentile_ci(&[]), (0.0, 0.0));
        assert_eq!(percentile_ci(&[0.9]), (0.9, 0.9));
        let (lo, hi) = percentile_ci(&[0.1, 0.5, 0.9]);
        assert!(lo <= hi);
    }

    #[test]
    fn resample_is_deterministic_and_with_replacement() {
        let mut r1 = SplitMix64::new(BOOTSTRAP_SEED);
        let mut r2 = SplitMix64::new(BOOTSTRAP_SEED);
        let a = resample_indices(&mut r1, 10);
        let b = resample_indices(&mut r2, 10);
        assert_eq!(a, b, "same seed must reproduce the resample");
        assert_eq!(a.len(), 10);
        assert!(a.iter().all(|&i| i < 10));
        // With replacement: a duplicate is overwhelmingly likely over a
        // 10-of-10 draw, and the deterministic seed makes this stable.
        let mut sorted = a.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert!(sorted.len() < a.len(), "expected a repeat: {a:?}");
        assert!(resample_indices(&mut r1, 0).is_empty());
    }

    #[test]
    fn fmt_ci_keeps_point_estimate_and_two_decimal_bounds() {
        assert_eq!(fmt_ci(0.929_4, (0.91, 0.95)), "0.929 [0.91, 0.95]");
    }

    #[test]
    fn bootstrap_cis_are_reproducible() {
        use crate::dataset::HallucinationClass as C;
        use crate::runner::{Outcome, Policy, RefusalReason, RunResult};
        let mk = |ex: &str, class, policy, refused| RunResult {
            example: ex.to_string(),
            class,
            policy,
            outcome: if refused {
                Outcome::Refused(RefusalReason::Unsupported)
            } else {
                Outcome::Accepted
            },
            expected: Outcome::Accepted,
            agreement: true,
        };
        let mut rows = Vec::new();
        for (ex, class) in [
            ("a", C::FabricatedSpan),
            ("b", C::OutOfContext),
            ("c", C::Contradicted),
            ("d", C::Valid),
        ] {
            for (p, refd) in [
                (Policy::VanillaRag, false),
                (Policy::ExistenceOnly, class == C::FabricatedSpan),
                (
                    Policy::TwoGate,
                    matches!(class, C::FabricatedSpan | C::OutOfContext),
                ),
                (Policy::ThreeGate, class != C::Valid),
            ] {
                rows.push(mk(ex, class, p, refd));
            }
        }
        let ex = group_examples(&rows);
        let a = bootstrap_f1_cis(&ex);
        let b = bootstrap_f1_cis(&ex);
        assert_eq!(a, b, "fixed-seed bootstrap must be reproducible");
        for (lo, hi) in a {
            assert!(lo <= hi, "CI lo>hi: [{lo}, {hi}]");
            assert!((0.0..=1.0).contains(&lo) && (0.0..=1.0).contains(&hi));
        }
    }
}

#[cfg(test)]
mod compare_tests {
    use super::*;
    use crate::dataset::HallucinationClass as C;
    use crate::runner::{Outcome, Policy, RefusalReason, RunResult};

    fn row(class: C, policy: Policy, agreement: bool) -> RunResult {
        RunResult {
            example: "ex".to_string(),
            class,
            policy,
            outcome: if agreement {
                Outcome::Accepted
            } else {
                Outcome::Refused(RefusalReason::Unsupported)
            },
            expected: Outcome::Accepted,
            agreement,
        }
    }

    #[test]
    fn compare_only_counts_three_gate_rows() {
        // Non-three-gate rows must not pollute the comparison.
        let base = vec![
            row(C::Valid, Policy::VanillaRag, false),
            row(C::Valid, Policy::ThreeGate, false),
        ];
        let cand = vec![
            row(C::Valid, Policy::VanillaRag, true),
            row(C::Valid, Policy::ThreeGate, true),
        ];
        let r = compare_report("distilbert", &base, "deberta", &cand);
        // Only the single three_gate row is counted per side.
        assert!(r.contains("| valid | 0 / 1 | 1 / 1 | +1 |"));
        assert!(r.contains("**all** | **0 / 1** | **1 / 1** | **+1**"));
    }

    #[test]
    fn headline_reports_valid_recovery() {
        let base = vec![
            row(C::Valid, Policy::ThreeGate, false),
            row(C::Valid, Policy::ThreeGate, false),
        ];
        let cand = vec![
            row(C::Valid, Policy::ThreeGate, true),
            row(C::Valid, Policy::ThreeGate, false),
        ];
        let r = compare_report("b", &base, "c", &cand);
        assert!(r.contains("Headline (valid-retention):** 0 / 2 → 1 / 2 (+1"));
    }

    #[test]
    fn structural_classes_show_zero_delta_when_model_independent() {
        let base = vec![
            row(C::FabricatedSpan, Policy::ThreeGate, true),
            row(C::OutOfContext, Policy::ThreeGate, true),
        ];
        let cand = base.clone();
        let r = compare_report("b", &base, "c", &cand);
        assert!(r.contains("| fabricated_span | 1 / 1 | 1 / 1 | 0 |"));
        assert!(r.contains("| out_of_context | 1 / 1 | 1 / 1 | 0 |"));
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;
    use crate::dataset::HallucinationClass as C;
    use crate::runner::{Outcome, Policy, RefusalReason, RunResult};

    fn r(example: &str, class: C, policy: Policy, refused: bool) -> RunResult {
        RunResult {
            example: example.to_string(),
            class,
            policy,
            outcome: if refused {
                Outcome::Refused(RefusalReason::Unsupported)
            } else {
                Outcome::Accepted
            },
            expected: Outcome::Accepted,
            agreement: true,
        }
    }

    // One example of each injected failure class, behaving exactly per
    // the policy matrix, plus one valid that the support gate
    // false-refuses.
    fn perfect_rows() -> Vec<RunResult> {
        let mut v = Vec::new();
        // FabricatedSpan: refused from existence onward.
        for (p, ref_) in [
            (Policy::VanillaRag, false),
            (Policy::ExistenceOnly, true),
            (Policy::TwoGate, true),
            (Policy::ThreeGate, true),
        ] {
            v.push(r("fab", C::FabricatedSpan, p, ref_));
        }
        // OutOfContext: refused from two-gate onward.
        for (p, ref_) in [
            (Policy::VanillaRag, false),
            (Policy::ExistenceOnly, false),
            (Policy::TwoGate, true),
            (Policy::ThreeGate, true),
        ] {
            v.push(r("ooc", C::OutOfContext, p, ref_));
        }
        // Contradicted: refused only at three-gate.
        for (p, ref_) in [
            (Policy::VanillaRag, false),
            (Policy::ExistenceOnly, false),
            (Policy::TwoGate, false),
            (Policy::ThreeGate, true),
        ] {
            v.push(r("con", C::Contradicted, p, ref_));
        }
        // Valid, but support gate false-refuses (the §5.1 FP).
        for (p, ref_) in [
            (Policy::VanillaRag, false),
            (Policy::ExistenceOnly, false),
            (Policy::TwoGate, false),
            (Policy::ThreeGate, true),
        ] {
            v.push(r("val", C::Valid, p, ref_));
        }
        v
    }

    #[test]
    fn structural_rules_are_perfect_and_disjoint() {
        let out = rule_metrics(&perfect_rows());
        // existence + in-context: precision 1.000 recall 1.000, F1 1.000
        // with its bootstrap CI appended. Point estimate is unchanged.
        assert!(out.contains("| existence | fabricated_span | 1.000 | 1.000 | 1.000 ["));
        assert!(out.contains("| in-context | out_of_context | 1.000 | 1.000 | 1.000 ["));
        // non-overlap: OOC 100% invisible to existence, support 100% to two-gate
        assert!(out.contains("accepted by existence-only: **1/1 (100.0%)**"));
        assert!(out.contains("accepted by two-gate: **1/1 (100.0%)**"));
    }

    #[test]
    fn support_false_refusal_shows_as_precision_loss() {
        // 1 true Contradicted caught + 1 valid false-refused => support
        // precision = 1/(1+1) = 0.5, recall = 1/1 = 1.0.
        let out = rule_metrics(&perfect_rows());
        assert!(
            out.contains("| support | unsupported+contradicted | 0.500 | 1.000 | 0.667 ["),
            "got:\n{out}"
        );
    }

    #[test]
    fn prf_math() {
        let m = Prf {
            tp: 3,
            fp: 1,
            fn_: 1,
        };
        assert!((m.precision() - 0.75).abs() < 1e-9);
        assert!((m.recall() - 0.75).abs() < 1e-9);
        assert!((m.f1() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn additive_lift_ladder() {
        // perfect_rows: 3 failures (fab, ooc, con) + 1 valid the
        // support gate false-refuses.
        // vanilla refuses nothing -> F1 0.
        // existence_only refuses fab only -> P 1.0 R 1/3 -> F1 0.500.
        // three_gate refuses all 3 + the valid (fp) -> P 0.75 R 1.0
        //   -> F1 0.857. Lift = (0.857 - 0.500) * 100 = +35.7.
        let out = rule_metrics(&perfect_rows());
        assert!(out.contains("## Claim 3 — additive lift"));
        assert!(
            out.contains("| existence_only | 1.000 | 0.333 | 0.500 | no |"),
            "got:\n{out}"
        );
        assert!(
            out.contains("| three_gate (1+2+3) | 0.750 | 1.000 | 0.857 [")
                && out.contains("] | yes |"),
            "got:\n{out}"
        );
        assert!(
            out.contains("**Additive lift: +35.7 F1 points**"),
            "got:\n{out}"
        );
        // Lift sentence keeps the point estimate and appends the CI.
        assert!(out.contains("(three-gate, F1 0.857 ["), "got:\n{out}");
    }
}
