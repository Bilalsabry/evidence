//! Bootstrap-dataset run-through. Asserts every (example, policy) row in
//! the bootstrap matches the [`expected_outcome`] matrix. A regression
//! in any gate's behavior will fail this test.

use evidence_eval::{load_dataset, run, Outcome, Policy, Report};

const BOOTSTRAP: &str = "datasets/bootstrap.toml";

#[test]
fn bootstrap_dataset_loads() {
    let ds = load_dataset(BOOTSTRAP).expect("load");
    assert!(!ds.examples.is_empty(), "bootstrap must have examples");
}

#[test]
fn every_row_matches_expected_matrix() {
    let ds = load_dataset(BOOTSTRAP).expect("load");
    let rows = run(&ds).expect("run");
    let mismatches: Vec<_> = rows.iter().filter(|r| !r.agreement).collect();
    assert!(
        mismatches.is_empty(),
        "rows disagreeing with the expected matrix: {:#?}",
        mismatches,
    );
}

#[test]
fn three_gate_catches_every_hallucination_class() {
    let ds = load_dataset(BOOTSTRAP).expect("load");
    let rows = run(&ds).expect("run");

    // For every example whose class is NOT `valid`, the three-gate policy
    // must refuse. This is the headline claim of the paper.
    let three_gate_rows = rows.iter().filter(|r| r.policy == Policy::ThreeGate);
    for row in three_gate_rows {
        match (row.class, &row.outcome) {
            (evidence_eval::HallucinationClass::Valid, Outcome::Accepted) => {}
            (evidence_eval::HallucinationClass::Valid, other) => {
                panic!(
                    "three-gate must accept valid examples; {} → {:?}",
                    row.example, other
                );
            }
            (_class, Outcome::Refused(_)) => {}
            (_class, Outcome::Accepted) => {
                panic!(
                    "three-gate must refuse non-valid examples; {} accepted instead",
                    row.example,
                );
            }
        }
    }
}

#[test]
fn vanilla_rag_accepts_everything() {
    let ds = load_dataset(BOOTSTRAP).expect("load");
    let rows = run(&ds).expect("run");
    for row in rows.iter().filter(|r| r.policy == Policy::VanillaRag) {
        assert!(
            matches!(row.outcome, Outcome::Accepted),
            "vanilla rag must accept every example, including hallucinations: {} → {:?}",
            row.example,
            row.outcome,
        );
    }
}

#[test]
fn report_renders_markdown() {
    let ds = load_dataset(BOOTSTRAP).expect("load");
    let rows = run(&ds).expect("run");
    let report = Report::new(rows);
    let md = evidence_eval::write_markdown(&report);
    assert!(md.contains("# Closed-Loop Citation eval"));
    assert!(md.contains("| class \\ policy |"));
    assert!(md.contains("Valid"));
    assert!(md.contains("Contradicted"));
}
