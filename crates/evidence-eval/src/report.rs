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
