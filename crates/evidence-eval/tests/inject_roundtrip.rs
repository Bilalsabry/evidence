//! Round-trip the failure-injection pipeline: load the bootstrap, inject
//! every valid example, then re-run the harness and assert each
//! injected variant behaves per its labeled class.

use evidence_eval::{
    inject_all_variants, load_dataset, run, Dataset, HallucinationClass, InjectionConfig, Outcome,
    Policy,
};

const BOOTSTRAP: &str = "datasets/bootstrap.toml";

#[test]
fn every_injected_variant_behaves_like_its_class() {
    let seeds = load_dataset(BOOTSTRAP).expect("load bootstrap");
    let config = InjectionConfig::default();

    let mut all = Vec::new();
    for ex in &seeds.examples {
        if ex.class != HallucinationClass::Valid {
            continue;
        }
        let variants =
            inject_all_variants(ex, &config).unwrap_or_else(|e| panic!("inject {}: {e}", ex.name));
        all.extend(variants);
    }
    assert!(
        !all.is_empty(),
        "expected at least one injected variant from the bootstrap",
    );

    // Build a derived dataset containing only the injected variants.
    let derived = Dataset { examples: all };
    let rows = run(&derived).expect("run injected dataset");
    let mismatches: Vec<_> = rows.iter().filter(|r| !r.agreement).collect();
    assert!(
        mismatches.is_empty(),
        "every injected variant must produce its labeled class outcome; \
         {} disagreements out of {} rows: {:#?}",
        mismatches.len(),
        rows.len(),
        mismatches,
    );
}

#[test]
fn every_injected_variant_is_caught_by_three_gate() {
    let seeds = load_dataset(BOOTSTRAP).expect("load");
    let config = InjectionConfig::default();
    let mut all = Vec::new();
    for ex in &seeds.examples {
        if ex.class != HallucinationClass::Valid {
            continue;
        }
        all.extend(inject_all_variants(ex, &config).unwrap());
    }
    let derived = Dataset { examples: all };
    let rows = run(&derived).expect("run");

    for row in rows.iter().filter(|r| r.policy == Policy::ThreeGate) {
        assert!(
            matches!(row.outcome, Outcome::Refused(_)),
            "three-gate must refuse every injected variant; {} → {:?}",
            row.example,
            row.outcome,
        );
    }
}

#[test]
fn injection_counts_match_per_seed_formula() {
    let seeds = load_dataset(BOOTSTRAP).expect("load");
    let config = InjectionConfig::default();

    let mut expected_total = 0usize;
    let mut actual_total = 0usize;
    for ex in &seeds.examples {
        if ex.class != HallucinationClass::Valid {
            continue;
        }
        // 2 structural variants (existence + in_context) + 1 per support_mutation.
        expected_total += 2 + ex.support_mutations.len();
        actual_total += inject_all_variants(ex, &config).unwrap().len();
    }
    assert_eq!(actual_total, expected_total);
}
