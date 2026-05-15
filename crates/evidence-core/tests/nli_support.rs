//! Real `NliCrossEncoder` integration test. First run downloads the
//! default NLI ONNX model (~265 MB) from the Hugging Face hub; subsequent
//! runs hit the cache. Exercises all three verdicts (`Supports`,
//! `Neutral`, `Contradicts`) on hand-picked sentence pairs so a regression
//! that breaks one verdict is loud.

use evidence_core::query::{NliCrossEncoder, NliSupportChecker, SupportChecker, SupportVerdict};

#[test]
fn nli_cross_encoder_classifies_all_three_verdicts() {
    let encoder = NliCrossEncoder::new().expect("downloading + loading the default NLI model");

    let pairs: &[(&str, &str, SupportVerdict, &str)] = &[
        (
            "The trial enrolled 240 patients with stage III non-small-cell lung cancer.",
            "The trial enrolled 240 patients.",
            SupportVerdict::Supports,
            "supporting pair",
        ),
        (
            "The trial enrolled 240 patients with stage III non-small-cell lung cancer.",
            "The trial enrolled 1000 patients.",
            SupportVerdict::Contradicts,
            "contradicting pair",
        ),
        (
            "The trial enrolled 240 patients with stage III non-small-cell lung cancer.",
            "The trial was funded by the National Cancer Institute.",
            SupportVerdict::Neutral,
            "off-topic pair",
        ),
    ];

    for (premise, hypothesis, expected, label) in pairs {
        let got = encoder.classify(premise, hypothesis).expect("classify");
        assert_eq!(
            got, *expected,
            "{label}: premise={premise:?} hypothesis={hypothesis:?} expected {expected:?} got {got:?}",
        );
    }
}

#[test]
fn nli_support_checker_aggregates_strict_wins() {
    let encoder = NliCrossEncoder::new().expect("nli encoder");
    let checker = NliSupportChecker::new(&encoder);

    // Two supporting + one contradicting → Contradicts must win.
    let sentence = "The trial enrolled 240 patients.";
    let cited = [
        "The study recruited 240 individuals with NSCLC.",
        "Two hundred and forty patients were entered into the protocol.",
        "The trial enrolled exactly 1000 patients.",
    ];
    let refs: Vec<&str> = cited.to_vec();
    let verdict = checker.check(sentence, &refs).expect("checker");
    assert_eq!(
        verdict,
        SupportVerdict::Contradicts,
        "strict-wins aggregation must surface Contradicts when any span contradicts",
    );

    // All neutral → Neutral.
    let neutral_cited = [
        "The sky is blue.",
        "The author received a grant from the NIH.",
    ];
    let neutral_refs: Vec<&str> = neutral_cited.to_vec();
    let v = checker.check(sentence, &neutral_refs).expect("checker");
    assert_eq!(v, SupportVerdict::Neutral);

    // All supporting → Supports.
    let supporting_cited = [
        "The trial recruited 240 patients with NSCLC.",
        "Two hundred and forty patients were enrolled.",
    ];
    let supporting_refs: Vec<&str> = supporting_cited.to_vec();
    let v = checker.check(sentence, &supporting_refs).expect("checker");
    assert_eq!(v, SupportVerdict::Supports);
}

#[test]
fn nli_support_checker_returns_neutral_for_no_citations() {
    let encoder = NliCrossEncoder::new().expect("nli encoder");
    let checker = NliSupportChecker::new(&encoder);
    let v = checker
        .check("anything goes", &[])
        .expect("empty cited_texts is documented as Neutral");
    assert_eq!(v, SupportVerdict::Neutral);
}
