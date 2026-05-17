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

/// Per-example refusal state across the four nested policies.
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
    // Existence: detector = refused by ExistenceOnly; owned = FabricatedSpan.
    let mut existence = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };
    // In-context: among ExistenceOnly-accepted, refused by TwoGate; owned = OOC.
    let mut incontext = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };
    // Support: among TwoGate-accepted, refused by ThreeGate; owned =
    // Unsupported + Contradicted.
    let mut support = Prf {
        tp: 0,
        fp: 0,
        fn_: 0,
    };

    for e in &ex {
        let r_exist = refused(e, Policy::ExistenceOnly);
        let r_two = refused(e, Policy::TwoGate);
        let r_three = refused(e, Policy::ThreeGate);

        // existence over all examples
        match (e.class == C::FabricatedSpan, r_exist) {
            (true, true) => existence.tp += 1,
            (true, false) => existence.fn_ += 1,
            (false, true) => existence.fp += 1,
            (false, false) => {}
        }
        // in-context: marginal, only where existence did not already fire
        if !r_exist {
            match (e.class == C::OutOfContext, r_two) {
                (true, true) => incontext.tp += 1,
                (true, false) => incontext.fn_ += 1,
                (false, true) => incontext.fp += 1,
                (false, false) => {}
            }
        }
        // support: marginal, only where two-gate accepted
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
        "| rule | owned class | precision | recall | F1 | tp/fp/fn |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    for (name, owned, m) in [
        ("existence", "fabricated_span", existence),
        ("in-context", "out_of_context", incontext),
        ("support", "unsupported+contradicted", support),
    ] {
        let _ = writeln!(
            out,
            "| {name} | {owned} | {:.3} | {:.3} | {:.3} | {}/{}/{} |",
            m.precision(),
            m.recall(),
            m.f1(),
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
        // existence + in-context: precision 1.000 recall 1.000 F1 1.000
        assert!(out.contains("| existence | fabricated_span | 1.000 | 1.000 | 1.000 |"));
        assert!(out.contains("| in-context | out_of_context | 1.000 | 1.000 | 1.000 |"));
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
            out.contains("| support | unsupported+contradicted | 0.500 | 1.000 |"),
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
}
