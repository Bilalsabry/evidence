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
