//! Dataset descriptive stats. Answers "is this dataset lopsided?"
//! during authoring, and emits the numbers the paper's dataset-card
//! section needs (class balance, per-example shape, what `inject` will
//! materialize).
//!
//! Pure over a parsed [`Dataset`] — no I/O, no PDFium.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::dataset::{Dataset, HallucinationClass};

/// Min / median / mean / max of a `usize` distribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dist {
    pub min: usize,
    pub median: f64,
    pub mean: f64,
    pub max: usize,
}

impl Dist {
    /// Summarize a slice. Empty input → all zeros.
    #[must_use]
    pub fn of(values: &[usize]) -> Self {
        if values.is_empty() {
            return Self {
                min: 0,
                median: 0.0,
                mean: 0.0,
                max: 0,
            };
        }
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let n = sorted.len();
        let median = if n % 2 == 1 {
            sorted[n / 2] as f64
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
        };
        let sum: usize = sorted.iter().sum();
        Self {
            min: sorted[0],
            median,
            mean: sum as f64 / n as f64,
            max: sorted[n - 1],
        }
    }
}

/// Everything `evidence-eval stats` reports.
#[derive(Debug, Clone)]
pub struct DatasetStats {
    pub total_examples: usize,
    pub class_counts: BTreeMap<HallucinationClass, usize>,
    pub corpus_spans: Dist,
    pub prompt_chunks: Dist,
    pub response_sentences: Dist,
    /// Valid seeds that carry at least one `support_mutations` entry.
    pub valid_with_mutations: usize,
    /// Total `support_mutations` across all valid seeds.
    pub support_mutations: usize,
}

impl DatasetStats {
    #[must_use]
    pub fn compute(dataset: &Dataset) -> Self {
        let mut class_counts: BTreeMap<HallucinationClass, usize> = BTreeMap::new();
        let mut corpus = Vec::with_capacity(dataset.examples.len());
        let mut chunks = Vec::with_capacity(dataset.examples.len());
        let mut sentences = Vec::with_capacity(dataset.examples.len());
        let mut valid_with_mutations = 0;
        let mut support_mutations = 0;

        for ex in &dataset.examples {
            *class_counts.entry(ex.class).or_insert(0) += 1;
            corpus.push(ex.corpus_spans.len());
            chunks.push(ex.prompt_chunks.len());
            sentences.push(ex.response_sentences.len());
            if ex.class == HallucinationClass::Valid && !ex.support_mutations.is_empty() {
                valid_with_mutations += 1;
            }
            support_mutations += ex.support_mutations.len();
        }

        Self {
            total_examples: dataset.examples.len(),
            class_counts,
            corpus_spans: Dist::of(&corpus),
            prompt_chunks: Dist::of(&chunks),
            response_sentences: Dist::of(&sentences),
            valid_with_mutations,
            support_mutations,
        }
    }

    /// Variants `evidence-eval inject` will emit: two structural
    /// injections (existence + in-context) per valid seed, plus one per
    /// `support_mutations` entry. Mirrors the formula the inject
    /// roundtrip test asserts.
    #[must_use]
    pub fn projected_injected(&self) -> usize {
        let valid = self
            .class_counts
            .get(&HallucinationClass::Valid)
            .copied()
            .unwrap_or(0);
        valid * 2 + self.support_mutations
    }
}

/// Render the stats as a markdown report.
#[must_use]
pub fn render_stats(stats: &DatasetStats) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Dataset stats");
    out.push('\n');
    let _ = writeln!(out, "- Examples: {}", stats.total_examples);
    let _ = writeln!(
        out,
        "- Valid seeds with support_mutations: {}",
        stats.valid_with_mutations
    );
    out.push('\n');

    let _ = writeln!(out, "## Class distribution");
    out.push('\n');
    let _ = writeln!(out, "| Class | Count | % |");
    let _ = writeln!(out, "|---|---|---|");
    for (class, count) in &stats.class_counts {
        let pct = if stats.total_examples == 0 {
            0.0
        } else {
            *count as f64 * 100.0 / stats.total_examples as f64
        };
        let _ = writeln!(out, "| {} | {count} | {pct:.1}% |", class_label(*class));
    }
    out.push('\n');

    let _ = writeln!(out, "## Per-example shape");
    out.push('\n');
    let _ = writeln!(out, "| Metric | min | median | mean | max |");
    let _ = writeln!(out, "|---|---|---|---|---|");
    write_dist_row(&mut out, "corpus_spans", &stats.corpus_spans);
    write_dist_row(&mut out, "prompt_chunks", &stats.prompt_chunks);
    write_dist_row(&mut out, "response_sentences", &stats.response_sentences);
    out.push('\n');

    let valid = stats
        .class_counts
        .get(&HallucinationClass::Valid)
        .copied()
        .unwrap_or(0);
    let _ = writeln!(out, "## Injection projection");
    out.push('\n');
    let _ = writeln!(out, "- valid seeds: {valid}");
    let _ = writeln!(out, "- structural variants (×2 per seed): {}", valid * 2);
    let _ = writeln!(
        out,
        "- support_mutation variants: {}",
        stats.support_mutations
    );
    let _ = writeln!(out, "- → injected total: {}", stats.projected_injected());
    let _ = writeln!(
        out,
        "- grand total after inject: {}",
        stats.total_examples + stats.projected_injected()
    );

    out
}

fn write_dist_row(out: &mut String, label: &str, d: &Dist) {
    let _ = writeln!(
        out,
        "| {label} | {} | {:.1} | {:.1} | {} |",
        d.min, d.median, d.mean, d.max
    );
}

fn class_label(class: HallucinationClass) -> &'static str {
    use HallucinationClass as C;
    match class {
        C::Valid => "valid",
        C::Uncited => "uncited",
        C::FabricatedSpan => "fabricated_span",
        C::OutOfContext => "out_of_context",
        C::Unsupported => "unsupported",
        C::Contradicted => "contradicted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(toml_src: &str) -> Dataset {
        toml::from_str(toml_src).unwrap()
    }

    const TWO_VALID_ONE_UNCITED: &str = r#"
[[example]]
name = "v1"
class = "valid"
corpus_spans = [{ id = 10, page = 1, text = "a" }, { id = 20, page = 1, text = "b" }]
prompt_chunks = [{ id = 1, span_range = [10, 20], text = "ab" }]
response_sentences = [{ text = "a", cited_spans = [10] }]
[[example.support_mutations]]
text = "not a"
class = "contradicted"

[[example]]
name = "v2"
class = "valid"
corpus_spans = [{ id = 10, page = 1, text = "c" }]
prompt_chunks = [{ id = 1, span_range = [10, 10], text = "c" }]
response_sentences = [{ text = "c", cited_spans = [10] }, { text = "c2", cited_spans = [10] }]

[[example]]
name = "u1"
class = "uncited"
corpus_spans = [{ id = 10, page = 1, text = "d" }]
prompt_chunks = [{ id = 1, span_range = [10, 10], text = "d" }]
response_sentences = [{ text = "d", cited_spans = [] }]
"#;

    #[test]
    fn dist_handles_even_and_odd_lengths() {
        assert_eq!(Dist::of(&[1, 2, 3]).median, 2.0);
        assert_eq!(Dist::of(&[1, 2, 3, 4]).median, 2.5);
        let d = Dist::of(&[2, 8, 2]);
        assert_eq!(d.min, 2);
        assert_eq!(d.max, 8);
        assert!((d.mean - 4.0).abs() < 1e-9);
    }

    #[test]
    fn dist_empty_is_zeros() {
        let d = Dist::of(&[]);
        assert_eq!(d.min, 0);
        assert_eq!(d.max, 0);
        assert_eq!(d.median, 0.0);
        assert_eq!(d.mean, 0.0);
    }

    #[test]
    fn counts_classes_and_shape() {
        let s = DatasetStats::compute(&ds(TWO_VALID_ONE_UNCITED));
        assert_eq!(s.total_examples, 3);
        assert_eq!(s.class_counts[&HallucinationClass::Valid], 2);
        assert_eq!(s.class_counts[&HallucinationClass::Uncited], 1);
        assert_eq!(s.corpus_spans.min, 1);
        assert_eq!(s.corpus_spans.max, 2);
        assert_eq!(s.response_sentences.max, 2);
    }

    #[test]
    fn tracks_support_mutations() {
        let s = DatasetStats::compute(&ds(TWO_VALID_ONE_UNCITED));
        assert_eq!(s.valid_with_mutations, 1);
        assert_eq!(s.support_mutations, 1);
    }

    #[test]
    fn injection_projection_matches_formula() {
        // 2 valid → 2*2 structural + 1 support_mutation = 5
        let s = DatasetStats::compute(&ds(TWO_VALID_ONE_UNCITED));
        assert_eq!(s.projected_injected(), 5);
        assert_eq!(s.total_examples + s.projected_injected(), 8);
    }

    #[test]
    fn render_includes_all_sections() {
        let s = DatasetStats::compute(&ds(TWO_VALID_ONE_UNCITED));
        let r = render_stats(&s);
        assert!(r.contains("# Dataset stats"));
        assert!(r.contains("## Class distribution"));
        assert!(r.contains("## Per-example shape"));
        assert!(r.contains("## Injection projection"));
        assert!(r.contains("| valid | 2 |"));
        assert!(r.contains("grand total after inject: 8"));
    }
}
