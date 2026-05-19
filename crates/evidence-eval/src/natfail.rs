//! §5.6 natural-failure study: annotation-worksheet preparation.
//!
//! This module is **preparatory only**. It does NOT grade anything and
//! it does NOT produce a result. Given a TOML of *real model-generated*
//! answers (the standard [`Example`] schema — `corpus_spans` /
//! `prompt_chunks` / `response_sentences` with the model's
//! `cited_spans`, `class` left as a to-be-human-labeled placeholder),
//! it runs every item through the validator via the existing policy
//! stack ([`run_with_model`]) and emits a markdown **worksheet**: the
//! validator's typed verdict in one column and a deliberately **blank
//! human-label column** for annotators to fill.
//!
//! The final natural-failure table (CLAIMS.md claim 4) is computed from
//! the *human* labels after double-annotation and adjudication — see
//! `docs/paper/natural-failure-protocol.md`. The harness must not
//! auto-grade the study against the validator; that would be circular
//! (validator vs. validator). The human label is the ground truth; the
//! validator verdict is the system under test.

use std::fmt::Write as _;

use crate::dataset::Dataset;
use crate::runner::{run_with_model, Outcome, Policy, RefusalReason, RunResult, SupportMode};

/// The validator's typed verdict for one worksheet row, projected from
/// the three-gate [`Outcome`] into the natural-failure protocol's
/// vocabulary (§7 of the protocol doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorVerdict {
    /// Three-gate validator accepted the (sentence, citation) claim.
    Accept,
    /// Refused: cited span had no citation at all.
    Uncited,
    /// Refused: cited span ID does not resolve in the corpus.
    UnknownSpan,
    /// Refused: cited span exists but was not in the prompt chunk set.
    OutOfContext,
    /// Refused: cited span was in context but does not entail the claim.
    Unsupported,
    /// Refused: cited span was in context but contradicts the claim.
    Contradicted,
    /// Any other refusal (e.g. a harness-setup error). Surfaced
    /// verbatim so the annotator sees something went wrong rather than
    /// a silently mislabeled row.
    Other,
}

impl ValidatorVerdict {
    /// Project a three-gate [`Outcome`] into a verdict. Only the
    /// three-gate policy's outcome is meaningful for this study — it is
    /// the full validator the natural-failure table scores.
    #[must_use]
    pub fn from_outcome(outcome: &Outcome) -> Self {
        match outcome {
            Outcome::Accepted => ValidatorVerdict::Accept,
            Outcome::Refused(reason) => match reason {
                RefusalReason::Uncited => ValidatorVerdict::Uncited,
                RefusalReason::UnknownSpan => ValidatorVerdict::UnknownSpan,
                RefusalReason::OutOfContext => ValidatorVerdict::OutOfContext,
                RefusalReason::Unsupported => ValidatorVerdict::Unsupported,
                RefusalReason::Contradicted => ValidatorVerdict::Contradicted,
                RefusalReason::Other(_) => ValidatorVerdict::Other,
            },
        }
    }

    /// Stable cell label for the worksheet.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            ValidatorVerdict::Accept => "accept",
            ValidatorVerdict::Uncited => "Uncited",
            ValidatorVerdict::UnknownSpan => "UnknownSpan",
            ValidatorVerdict::OutOfContext => "OutOfContext",
            ValidatorVerdict::Unsupported => "Unsupported",
            ValidatorVerdict::Contradicted => "Contradicted",
            ValidatorVerdict::Other => "other",
        }
    }
}

/// One worksheet row: a candidate item and what the three-gate
/// validator did with it. The human label is intentionally absent — it
/// is filled by annotators on the rendered worksheet, never here.
#[derive(Debug, Clone)]
pub struct WorksheetRow {
    pub example: String,
    pub verdict: ValidatorVerdict,
}

/// Build the worksheet rows from validator [`RunResult`]s. Only the
/// three-gate policy's result is kept per example — the full validator
/// is what the natural-failure table scores; the weaker policies are
/// run by the harness but are not part of this study's worksheet.
///
/// Input order is preserved (the first three-gate row seen per example
/// wins, matching `run_with_model`'s stable ordering).
#[must_use]
pub fn worksheet_rows(results: &[RunResult]) -> Vec<WorksheetRow> {
    results
        .iter()
        .filter(|r| r.policy == Policy::ThreeGate)
        .map(|r| WorksheetRow {
            example: r.example.clone(),
            verdict: ValidatorVerdict::from_outcome(&r.outcome),
        })
        .collect()
}

/// Render the markdown annotation worksheet. This is the **scoring
/// copy** described in §7 of the protocol: it carries the validator
/// verdict column. The annotation copy (validator column blanked) is a
/// manual, documented downstream step — deliberately NOT a harness
/// feature, so the harness only ever emits a worksheet, never a graded
/// result.
///
/// Pure over the rows so it is unit-testable without running a model.
#[must_use]
pub fn render_worksheet(rows: &[WorksheetRow], answers_label: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Natural-failure annotation worksheet (§5.6)\n");
    let _ = writeln!(
        out,
        "Source answers: `{answers_label}`. {} candidate items run \
         through the three-gate validator.\n",
        rows.len()
    );
    let _ = writeln!(
        out,
        "**This is a worksheet, NOT a result.** The validator verdict \
         column is the *system under test*; the blank human-label column \
         is the *ground truth*, filled independently by two annotators \
         and adjudicated per `docs/paper/natural-failure-protocol.md`. \
         The harness does NOT auto-grade this study.\n"
    );

    let _ = writeln!(
        out,
        "| # | example | validator verdict | human label | annotator notes |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for (i, row) in rows.iter().enumerate() {
        let _ = writeln!(
            out,
            "| {} | {} | {} |  |  |",
            i + 1,
            row.example,
            row.verdict.label(),
        );
    }

    let _ = writeln!(
        out,
        "\n> Human-label values: one of `valid`, `uncited`, \
         `fabricated_span`, `out_of_context`, `unsupported`, \
         `contradicted`, `partial_support`, `ambiguous` \
         (see protocol §5). Leave the validator-verdict column intact in \
         the scoring copy; blank it in the annotation copy so it does \
         not anchor annotators. No precision/recall is computed here — \
         that table is built from adjudicated human labels only."
    );
    out
}

/// Run a candidate-answer dataset through the validator and render the
/// worksheet. The dataset's `class` fields are placeholders (to be
/// human-labeled) and are *not* consulted for the verdict — the verdict
/// comes from the validator's behavior on the model's actual citations.
///
/// Under [`SupportMode::RealNli`] the support gate uses a real NLI
/// checkpoint (the natural-failure study's intended mode — real model
/// outputs deserve the real support gate). [`SupportMode::Mock`] is
/// accepted for deterministic tests but is class-derived and therefore
/// only meaningful when the placeholder class happens to be set; the
/// CLI defaults to real-NLI for this reason.
///
/// # Errors
///
/// Propagates dataset-seeding errors and, under `RealNli`, NLI
/// initialization failures.
pub fn prepare_worksheet(
    dataset: &Dataset,
    mode: SupportMode,
    nli_model: Option<&str>,
    answers_label: &str,
) -> anyhow::Result<String> {
    let results = run_with_model(dataset, mode, nli_model)?;
    let rows = worksheet_rows(&results);
    Ok(render_worksheet(&rows, answers_label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::HallucinationClass;

    fn result(example: &str, policy: Policy, outcome: Outcome) -> RunResult {
        RunResult {
            example: example.to_string(),
            class: HallucinationClass::Valid,
            policy,
            outcome: outcome.clone(),
            expected: outcome,
            agreement: true,
        }
    }

    #[test]
    fn verdict_projects_every_refusal_reason() {
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Accepted),
            ValidatorVerdict::Accept
        );
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Refused(RefusalReason::Uncited)),
            ValidatorVerdict::Uncited
        );
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Refused(RefusalReason::UnknownSpan)),
            ValidatorVerdict::UnknownSpan
        );
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Refused(RefusalReason::OutOfContext)),
            ValidatorVerdict::OutOfContext
        );
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Refused(RefusalReason::Unsupported)),
            ValidatorVerdict::Unsupported
        );
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Refused(RefusalReason::Contradicted)),
            ValidatorVerdict::Contradicted
        );
        assert_eq!(
            ValidatorVerdict::from_outcome(&Outcome::Refused(RefusalReason::Other(
                "seed error".into()
            ))),
            ValidatorVerdict::Other
        );
    }

    #[test]
    fn worksheet_rows_keep_only_three_gate() {
        let results = vec![
            result("a", Policy::VanillaRag, Outcome::Accepted),
            result("a", Policy::ExistenceOnly, Outcome::Accepted),
            result("a", Policy::TwoGate, Outcome::Accepted),
            result(
                "a",
                Policy::ThreeGate,
                Outcome::Refused(RefusalReason::Unsupported),
            ),
            result("b", Policy::ThreeGate, Outcome::Accepted),
        ];
        let rows = worksheet_rows(&results);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].example, "a");
        assert_eq!(rows[0].verdict, ValidatorVerdict::Unsupported);
        assert_eq!(rows[1].example, "b");
        assert_eq!(rows[1].verdict, ValidatorVerdict::Accept);
    }

    #[test]
    fn render_has_blank_human_column_and_no_metrics() {
        let rows = vec![
            WorksheetRow {
                example: "natfail_001".into(),
                verdict: ValidatorVerdict::Unsupported,
            },
            WorksheetRow {
                example: "natfail_002".into(),
                verdict: ValidatorVerdict::Accept,
            },
        ];
        let md = render_worksheet(&rows, "candidate_answers.toml");

        assert!(md.contains("# Natural-failure annotation worksheet (§5.6)"));
        assert!(md.contains("`candidate_answers.toml`"));
        assert!(md.contains("2 candidate items"));
        assert!(md.contains("worksheet, NOT a result"));
        assert!(md.contains("does NOT auto-grade"));
        // Header + a blank human-label cell per row.
        assert!(md.contains("| # | example | validator verdict | human label | annotator notes |"));
        assert!(md.contains("| 1 | natfail_001 | Unsupported |  |  |"));
        assert!(md.contains("| 2 | natfail_002 | accept |  |  |"));
        // No fabricated metrics anywhere.
        assert!(!md.to_lowercase().contains("precision ="));
        assert!(!md.to_lowercase().contains("recall ="));
        assert!(!md.contains("F1"));
    }

    #[test]
    fn render_handles_empty_input() {
        let md = render_worksheet(&[], "empty.toml");
        assert!(md.contains("0 candidate items"));
        assert!(md.contains("| # | example | validator verdict | human label | annotator notes |"));
    }
}
