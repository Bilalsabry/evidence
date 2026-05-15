//! Dataset linter — catches authoring mistakes before the harness runs.
//!
//! Each check has a name and a severity. **Errors** mean the dataset
//! will mis-behave in the runner (e.g., the example's class doesn't
//! match its construction); **warnings** mean the dataset is internally
//! consistent but probably not what the author intended (e.g., an
//! `unsupported` example that names a span the model wasn't shown —
//! that should be `out_of_context`, not `unsupported`).
//!
//! Run via `evidence-eval lint <dataset.toml>`. Exit 0 if clean, 2 if
//! any errors, 0 if only warnings.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::dataset::{Dataset, Example, HallucinationClass};

/// One lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub example: Option<String>,
    pub check: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The dataset will mis-behave at runtime (or already does).
    Error,
    /// Internal consistency holds, but the example probably doesn't
    /// demonstrate what its class claims.
    Warning,
}

/// Run every lint check on a dataset. Returns a `Vec<Diagnostic>`;
/// callers decide how to render and what exit code to use.
#[must_use]
pub fn lint(dataset: &Dataset) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_duplicate_example_names(dataset, &mut diags);
    for example in &dataset.examples {
        check_example(example, &mut diags);
    }
    diags
}

fn push(
    diags: &mut Vec<Diagnostic>,
    severity: Severity,
    example: Option<&str>,
    check: &'static str,
    message: String,
) {
    diags.push(Diagnostic {
        severity,
        example: example.map(str::to_string),
        check,
        message,
    });
}

fn check_duplicate_example_names(dataset: &Dataset, diags: &mut Vec<Diagnostic>) {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for ex in &dataset.examples {
        *seen.entry(ex.name.as_str()).or_insert(0) += 1;
    }
    for (name, count) in seen {
        if count > 1 {
            push(
                diags,
                Severity::Error,
                Some(name),
                "duplicate_example_name",
                format!("example name '{name}' appears {count} times"),
            );
        }
    }
}

fn check_example(example: &Example, diags: &mut Vec<Diagnostic>) {
    let name = Some(example.name.as_str());

    // Structural checks (apply to every class).
    let corpus_ids: BTreeSet<i64> = example.corpus_spans.iter().map(|s| s.id).collect();
    if corpus_ids.len() != example.corpus_spans.len() {
        push(
            diags,
            Severity::Error,
            name,
            "duplicate_corpus_span_id",
            "corpus_spans contains duplicate IDs".to_string(),
        );
    }

    for chunk in &example.prompt_chunks {
        if chunk.span_range[0] > chunk.span_range[1] {
            push(
                diags,
                Severity::Error,
                name,
                "invalid_span_range",
                format!(
                    "prompt_chunk {} has span_range [{}, {}] where start > end",
                    chunk.id, chunk.span_range[0], chunk.span_range[1],
                ),
            );
        }
    }

    if example.response_sentences.is_empty() {
        push(
            diags,
            Severity::Error,
            name,
            "empty_response",
            "response_sentences is empty".to_string(),
        );
        // Nothing more to check for this example.
        return;
    }

    // The set of spans the model "saw" (the union of prompt_chunks' ranges).
    let in_context: Vec<i64> = example
        .prompt_chunks
        .iter()
        .flat_map(|c| c.span_range[0]..=c.span_range[1])
        .collect();
    let in_context_set: BTreeSet<i64> = in_context.into_iter().collect();

    // Author-intent checks per class.
    match example.class {
        HallucinationClass::Valid => {
            // Every sentence must cite at least one span; every cited
            // span must be in corpus AND in the prompt's allowed range
            // AND not contradict the sentence (NLI's job; we don't
            // check semantically here, but we do check structure).
            for (idx, sentence) in example.response_sentences.iter().enumerate() {
                if sentence.cited_spans.is_empty() {
                    push(
                        diags,
                        Severity::Error,
                        name,
                        "valid_uncited_sentence",
                        format!("class=valid but response_sentences[{idx}] has empty cited_spans"),
                    );
                }
                for cite in &sentence.cited_spans {
                    if !corpus_ids.contains(cite) {
                        push(
                            diags,
                            Severity::Error,
                            name,
                            "valid_unknown_cite",
                            format!("class=valid but cited span {cite} is not in corpus_spans"),
                        );
                    }
                    if !in_context_set.contains(cite) {
                        push(
                            diags,
                            Severity::Error,
                            name,
                            "valid_out_of_context_cite",
                            format!(
                                "class=valid but cited span {cite} is not in any prompt_chunk's span_range"
                            ),
                        );
                    }
                }
            }
            // Mutation-class sanity: every support_mutation must be
            // unsupported or contradicted.
            for (idx, mut_) in example.support_mutations.iter().enumerate() {
                if !matches!(
                    mut_.class,
                    HallucinationClass::Unsupported | HallucinationClass::Contradicted
                ) {
                    push(
                        diags,
                        Severity::Error,
                        name,
                        "bad_support_mutation_class",
                        format!(
                            "support_mutations[{idx}] has class {:?}; must be unsupported or contradicted",
                            mut_.class
                        ),
                    );
                }
                if mut_.text.trim().is_empty() {
                    push(
                        diags,
                        Severity::Error,
                        name,
                        "empty_support_mutation",
                        format!("support_mutations[{idx}] has empty text"),
                    );
                }
            }
        }

        HallucinationClass::Uncited => {
            // At least one sentence must have empty cited_spans.
            let has_uncited = example
                .response_sentences
                .iter()
                .any(|s| s.cited_spans.is_empty());
            if !has_uncited {
                push(
                    diags,
                    Severity::Error,
                    name,
                    "uncited_has_all_citations",
                    "class=uncited but every response_sentence has citations".to_string(),
                );
            }
        }

        HallucinationClass::FabricatedSpan => {
            // At least one cite must reference a span NOT in corpus.
            let all_cites: Vec<i64> = example
                .response_sentences
                .iter()
                .flat_map(|s| s.cited_spans.iter().copied())
                .collect();
            let has_fabricated = all_cites.iter().any(|c| !corpus_ids.contains(c));
            if !has_fabricated {
                push(
                    diags,
                    Severity::Error,
                    name,
                    "fabricated_span_no_fabrication",
                    "class=fabricated_span but every cited span exists in corpus_spans".to_string(),
                );
            }
        }

        HallucinationClass::OutOfContext => {
            // At least one cite must reference a span that IS in corpus
            // but is NOT in any prompt_chunk's span_range.
            let all_cites: Vec<i64> = example
                .response_sentences
                .iter()
                .flat_map(|s| s.cited_spans.iter().copied())
                .collect();
            let has_ooc = all_cites
                .iter()
                .any(|c| corpus_ids.contains(c) && !in_context_set.contains(c));
            if !has_ooc {
                push(
                    diags,
                    Severity::Warning,
                    name,
                    "out_of_context_no_ooc_cite",
                    "class=out_of_context but no cited span is in corpus AND outside the prompt's chunks; \
                     either the example doesn't demonstrate its class or it would be caught by gate 1 instead".to_string(),
                );
            }
        }

        HallucinationClass::Unsupported | HallucinationClass::Contradicted => {
            // Every cite must be in corpus AND in the prompt's range
            // (otherwise gate 1 or 2 catches first, defeating the test).
            for sentence in &example.response_sentences {
                for cite in &sentence.cited_spans {
                    if !corpus_ids.contains(cite) {
                        push(
                            diags,
                            Severity::Warning,
                            name,
                            "support_class_unknown_cite",
                            format!(
                                "class={:?} but cited span {cite} is not in corpus_spans; \
                                 gate 1 (UnknownSpan) will catch this before gate 3 has a chance",
                                example.class
                            ),
                        );
                    } else if !in_context_set.contains(cite) {
                        push(
                            diags,
                            Severity::Warning,
                            name,
                            "support_class_out_of_context",
                            format!(
                                "class={:?} but cited span {cite} is in corpus yet outside any prompt_chunk; \
                                 gate 2 (OutOfContext) will catch this before gate 3 has a chance",
                                example.class
                            ),
                        );
                    }
                }
            }
        }
    }

    // Cross-class invariants: support_mutations only on Valid examples.
    if example.class != HallucinationClass::Valid && !example.support_mutations.is_empty() {
        push(
            diags,
            Severity::Warning,
            name,
            "support_mutations_on_non_valid",
            format!(
                "support_mutations are only used by the injection harness on Valid seeds; \
                 they're ignored on class={:?}",
                example.class
            ),
        );
    }
}

/// Format a list of diagnostics as a human-readable report (markdown
/// table). Designed for the CLI's terminal output.
#[must_use]
pub fn render_report(diags: &[Diagnostic]) -> String {
    if diags.is_empty() {
        return "lint: 0 diagnostics. Dataset is clean.\n".to_string();
    }
    let mut out = String::new();
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let _ = writeln!(out, "lint: {errors} error(s), {warnings} warning(s).\n");
    let _ = writeln!(out, "| severity | example | check | message |");
    let _ = writeln!(out, "|---|---|---|---|");
    let mut sorted: Vec<&Diagnostic> = diags.iter().collect();
    sorted.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.example.cmp(&b.example))
            .then_with(|| a.check.cmp(b.check))
    });
    for d in sorted {
        let severity = match d.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "warn",
        };
        let example = d.example.as_deref().unwrap_or("—");
        let _ = writeln!(
            out,
            "| {severity} | {example} | {} | {} |",
            d.check, d.message
        );
    }
    out
}

/// Convenience: are there any errors (non-warning diagnostics)?
#[must_use]
pub fn has_errors(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.severity == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{CorpusSpan, PromptChunk, ResponseSentence, SupportMutation};

    fn valid_example() -> Example {
        Example {
            name: "v".to_string(),
            class: HallucinationClass::Valid,
            description: String::new(),
            corpus_spans: vec![CorpusSpan {
                id: 1,
                page: 1,
                text: "hi".to_string(),
            }],
            prompt_chunks: vec![PromptChunk {
                id: 100,
                span_range: [1, 1],
                text: "hi".to_string(),
            }],
            response_sentences: vec![ResponseSentence {
                text: "hi".to_string(),
                cited_spans: vec![1],
            }],
            support_mutations: vec![],
        }
    }

    fn lint_one(example: Example) -> Vec<Diagnostic> {
        lint(&Dataset {
            examples: vec![example],
        })
    }

    #[test]
    fn clean_valid_example_has_no_diagnostics() {
        assert!(lint_one(valid_example()).is_empty());
    }

    #[test]
    fn duplicate_example_names_are_errors() {
        let a = valid_example();
        let b = valid_example(); // same name "v"
        let diags = lint(&Dataset {
            examples: vec![a, b],
        });
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].check, "duplicate_example_name");
    }

    #[test]
    fn duplicate_corpus_span_id_is_an_error() {
        let mut ex = valid_example();
        ex.corpus_spans.push(CorpusSpan {
            id: 1, // already present
            page: 2,
            text: "dup".to_string(),
        });
        let diags = lint_one(ex);
        assert!(diags.iter().any(|d| d.check == "duplicate_corpus_span_id"));
    }

    #[test]
    fn invalid_span_range_is_an_error() {
        let mut ex = valid_example();
        ex.prompt_chunks[0].span_range = [5, 1]; // backwards
        let diags = lint_one(ex);
        assert!(diags.iter().any(|d| d.check == "invalid_span_range"));
    }

    #[test]
    fn empty_response_is_an_error() {
        let mut ex = valid_example();
        ex.response_sentences.clear();
        let diags = lint_one(ex);
        assert!(diags.iter().any(|d| d.check == "empty_response"));
    }

    #[test]
    fn valid_uncited_sentence_is_an_error() {
        let mut ex = valid_example();
        ex.response_sentences[0].cited_spans.clear();
        let diags = lint_one(ex);
        assert!(diags.iter().any(|d| d.check == "valid_uncited_sentence"));
    }

    #[test]
    fn valid_cite_outside_prompt_is_an_error() {
        let mut ex = valid_example();
        // Add a corpus span 99 but only show span 1 in prompt; cite 99.
        ex.corpus_spans.push(CorpusSpan {
            id: 99,
            page: 2,
            text: "x".to_string(),
        });
        ex.response_sentences[0].cited_spans = vec![99];
        let diags = lint_one(ex);
        assert!(diags.iter().any(|d| d.check == "valid_out_of_context_cite"));
    }

    #[test]
    fn fabricated_class_with_only_real_cites_is_an_error() {
        let mut ex = valid_example();
        ex.class = HallucinationClass::FabricatedSpan;
        // cited_spans still [1] — which IS in corpus, so no fabrication.
        let diags = lint_one(ex);
        assert!(diags
            .iter()
            .any(|d| d.check == "fabricated_span_no_fabrication"));
    }

    #[test]
    fn out_of_context_class_without_ooc_cite_warns() {
        let mut ex = valid_example();
        ex.class = HallucinationClass::OutOfContext;
        // cited_spans still [1] — which IS in the prompt's range, so
        // not out-of-context. Should warn.
        let diags = lint_one(ex);
        assert!(diags
            .iter()
            .any(|d| d.check == "out_of_context_no_ooc_cite" && d.severity == Severity::Warning));
    }

    #[test]
    fn unsupported_with_unknown_cite_warns_about_gate_1() {
        let mut ex = valid_example();
        ex.class = HallucinationClass::Unsupported;
        ex.response_sentences[0].cited_spans = vec![999]; // not in corpus
        let diags = lint_one(ex);
        assert!(diags
            .iter()
            .any(|d| d.check == "support_class_unknown_cite" && d.severity == Severity::Warning));
    }

    #[test]
    fn support_mutation_on_non_valid_warns() {
        let mut ex = valid_example();
        ex.class = HallucinationClass::Contradicted;
        // Make it a structurally valid contradicted example.
        ex.support_mutations = vec![SupportMutation {
            text: "x".to_string(),
            class: HallucinationClass::Contradicted,
        }];
        let diags = lint_one(ex);
        assert!(diags
            .iter()
            .any(|d| d.check == "support_mutations_on_non_valid"));
    }

    #[test]
    fn bad_support_mutation_class_is_an_error() {
        let mut ex = valid_example();
        ex.support_mutations = vec![SupportMutation {
            text: "x".to_string(),
            class: HallucinationClass::FabricatedSpan, // not allowed
        }];
        let diags = lint_one(ex);
        assert!(diags
            .iter()
            .any(|d| d.check == "bad_support_mutation_class"));
    }

    #[test]
    fn render_report_handles_empty_input() {
        let s = render_report(&[]);
        assert!(s.contains("0 diagnostics"));
    }

    #[test]
    fn render_report_sorts_errors_before_warnings() {
        let diags = vec![
            Diagnostic {
                severity: Severity::Warning,
                example: Some("a".to_string()),
                check: "x",
                message: "warn".to_string(),
            },
            Diagnostic {
                severity: Severity::Error,
                example: Some("b".to_string()),
                check: "y",
                message: "err".to_string(),
            },
        ];
        let s = render_report(&diags);
        let err_pos = s.find("ERROR").unwrap();
        let warn_pos = s.find("| warn ").unwrap();
        assert!(err_pos < warn_pos);
    }
}
