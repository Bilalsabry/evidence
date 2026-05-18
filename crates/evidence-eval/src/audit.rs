//! Faithfulness audit: verify every cited corpus span in a benchmark
//! dataset is real source text.
//!
//! An AI-authored benchmark's single largest credibility risk is an
//! *invented* corpus span — text that reads like a drug label but
//! appears nowhere in the source PDF. A reviewer who finds one
//! fabricated span discounts the whole dataset. This module mechanically
//! checks every `corpus_span.text` against the concatenated, normalized
//! text of the corpus PDFs and reports anything that isn't verbatim
//! (or hyphen/space-insensitively) present.
//!
//! The pure classification logic is I/O-free and unit-tested; the CLI
//! wrapper (in `main.rs`) supplies the corpus text.

use std::fmt::Write as _;

use crate::Dataset;

/// How a span's text relates to the corpus source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// `normalize(span)` is a substring of the normalized corpus.
    Exact,
    /// Only matches after additionally stripping spaces and `-`
    /// (handles hyphenation / OCR splits like
    /// "life-threatening" vs "lifethreatening").
    Fuzzy,
    /// Not found even after hyphen/space stripping. Requires manual
    /// review — may be a genuine fabrication, or an artifact of
    /// aggressive normalization.
    Missing,
}

/// Normalize text for robust substring matching.
///
/// Lowercases, maps curly quotes/dashes to ASCII, collapses every run
/// of whitespace (including `\r\n`) to a single space, and trims. This
/// makes matching insensitive to the cosmetic differences that PDF text
/// extraction routinely introduces.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            in_ws = true;
            continue;
        }
        if in_ws {
            if !out.is_empty() {
                out.push(' ');
            }
            in_ws = false;
        }
        let mapped = match ch {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{2032}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{2033}' => '"',
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => '-',
            other => other,
        };
        for lc in mapped.to_lowercase() {
            out.push(lc);
        }
    }
    out
}

/// Strip every space and `-` from already-normalized text.
fn strip_hyphen_space(norm: &str) -> String {
    norm.chars().filter(|c| *c != ' ' && *c != '-').collect()
}

/// Classify a single span against the (already normalized) corpus text.
///
/// `corpus_norm` must be the output of [`normalize`] over the
/// concatenated corpus. The span is normalized here.
pub fn match_in(corpus_norm: &str, span: &str) -> MatchKind {
    let span_norm = normalize(span);
    if span_norm.is_empty() {
        // An empty span trivially "occurs" everywhere; treat as Exact so
        // it isn't flagged as a fabrication.
        return MatchKind::Exact;
    }
    if corpus_norm.contains(&span_norm) {
        return MatchKind::Exact;
    }
    let corpus_stripped = strip_hyphen_space(corpus_norm);
    let span_stripped = strip_hyphen_space(&span_norm);
    if !span_stripped.is_empty() && corpus_stripped.contains(&span_stripped) {
        return MatchKind::Fuzzy;
    }
    MatchKind::Missing
}

fn truncate(s: &str, max: usize) -> String {
    let one_line: String = s
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let collapsed = one_line.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max {
        collapsed
    } else {
        let kept: String = collapsed.chars().take(max.saturating_sub(1)).collect();
        format!("{kept}…")
    }
}

/// Audit every corpus span in `dataset` against `corpus_norm` and render
/// a markdown report. `corpus_norm` must be [`normalize`]d corpus text.
pub fn audit_report(dataset: &Dataset, corpus_norm: &str) -> String {
    let mut exact = 0usize;
    let mut fuzzy = 0usize;
    let mut missing = 0usize;
    let mut total_spans = 0usize;

    struct Flagged {
        example: String,
        text: String,
        kind: MatchKind,
    }
    let mut flagged: Vec<Flagged> = Vec::new();

    for ex in &dataset.examples {
        for span in &ex.corpus_spans {
            total_spans += 1;
            match match_in(corpus_norm, &span.text) {
                MatchKind::Exact => exact += 1,
                MatchKind::Fuzzy => {
                    fuzzy += 1;
                    flagged.push(Flagged {
                        example: ex.name.clone(),
                        text: span.text.clone(),
                        kind: MatchKind::Fuzzy,
                    });
                }
                MatchKind::Missing => {
                    missing += 1;
                    flagged.push(Flagged {
                        example: ex.name.clone(),
                        text: span.text.clone(),
                        kind: MatchKind::Missing,
                    });
                }
            }
        }
    }

    let mut md = String::new();
    let _ = writeln!(md, "# Faithfulness audit");
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "Verifies every cited corpus span is real source text — the \
         primary reviewer attack on an AI-authored benchmark."
    );
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "- **Missing** = not found verbatim or hyphen-insensitively in \
         any corpus PDF — requires manual review; may be genuine \
         fabrication OR aggressive normalization."
    );
    let _ = writeln!(
        md,
        "- **Fuzzy** = found only after hyphen/space stripping (likely \
         benign OCR/hyphenation)."
    );
    let _ = writeln!(md);
    let _ = writeln!(md, "## Totals");
    let _ = writeln!(md);
    let _ = writeln!(md, "- examples: {}", dataset.examples.len());
    let _ = writeln!(md, "- corpus spans: {total_spans}");
    let _ = writeln!(md, "- exact: {exact}");
    let _ = writeln!(md, "- fuzzy: {fuzzy}");
    let _ = writeln!(md, "- missing: {missing}");
    let _ = writeln!(md);

    let _ = writeln!(md, "## Flagged spans");
    let _ = writeln!(md);
    if flagged.is_empty() {
        let _ = writeln!(md, "_None — every corpus span is verbatim in the source._");
    } else {
        let _ = writeln!(md, "| example | span text (≤120 chars) | kind |");
        let _ = writeln!(md, "| --- | --- | --- |");
        for f in &flagged {
            let kind = match f.kind {
                MatchKind::Exact => "exact",
                MatchKind::Fuzzy => "fuzzy",
                MatchKind::Missing => "missing",
            };
            let cell = truncate(&f.text, 120).replace('|', "\\|");
            let name = f.example.replace('|', "\\|");
            let _ = writeln!(md, "| {name} | {cell} | {kind} |");
        }
    }
    let _ = writeln!(md);

    md
}

/// Whether the audit found any [`MatchKind::Missing`] span. The CLI uses
/// this to choose its exit code (mirrors `lint`'s convention).
pub fn has_missing(dataset: &Dataset, corpus_norm: &str) -> bool {
    dataset
        .examples
        .iter()
        .flat_map(|e| &e.corpus_spans)
        .any(|s| match_in(corpus_norm, &s.text) == MatchKind::Missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_whitespace_and_quotes() {
        let input = "  The  patient\r\n\tshould\u{2019}t   take  it. ";
        assert_eq!(normalize(input), "the patient should't take it.");
    }

    #[test]
    fn normalize_maps_curly_quotes_and_dashes() {
        let input = "\u{201C}life\u{2014}threatening\u{201D}";
        assert_eq!(normalize(input), "\"life-threatening\"");
    }

    #[test]
    fn match_exact_for_verbatim() {
        let corpus = normalize("Do not use during pregnancy. It is contraindicated.");
        assert_eq!(
            match_in(&corpus, "It is contraindicated."),
            MatchKind::Exact
        );
    }

    #[test]
    fn match_exact_ignores_cosmetic_differences() {
        let corpus = normalize("The reaction was life-threatening in trials.");
        assert_eq!(
            match_in(&corpus, "the   reaction\nwas LIFE-threatening"),
            MatchKind::Exact
        );
    }

    #[test]
    fn match_fuzzy_for_hyphenation_split() {
        // Corpus has the OCR-joined form; span uses the hyphenated form.
        let corpus = normalize("A serious lifethreatening event occurred.");
        assert_eq!(match_in(&corpus, "life-threatening"), MatchKind::Fuzzy);
    }

    #[test]
    fn match_missing_for_invented_text() {
        let corpus = normalize("The label discusses dosage and contraindications.");
        assert_eq!(
            match_in(&corpus, "This drug cures every known disease instantly."),
            MatchKind::Missing
        );
    }

    #[test]
    fn empty_span_is_exact() {
        let corpus = normalize("anything");
        assert_eq!(match_in(&corpus, "   "), MatchKind::Exact);
    }
}
