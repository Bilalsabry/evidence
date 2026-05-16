//! TOML dataset format for evaluation examples.
//!
//! Each example specifies a small corpus, the chunks shown to the model
//! (a subset of that corpus), the model's fixture response, and a class
//! label that tags what hallucination the example demonstrates.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// What an example demonstrates. The runner derives the expected
/// behavior per [`crate::Policy`] from this tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HallucinationClass {
    /// A well-formed, in-context, supported sentence. All policies accept.
    Valid,
    /// Model produced a sentence with no citation. Rejected by anything
    /// that requires citations.
    Uncited,
    /// Model invented a span ID that doesn't exist in the DB.
    FabricatedSpan,
    /// Model cited a real span that wasn't in the prompt's chunk set.
    OutOfContext,
    /// Cited span exists, was in context, but doesn't entail the
    /// sentence (neutral).
    Unsupported,
    /// Cited span exists, was in context, but contradicts the sentence.
    Contradicted,
}

/// A single span in the corpus.
#[derive(Debug, Clone, Deserialize)]
pub struct CorpusSpan {
    pub id: i64,
    pub page: u32,
    pub text: String,
}

/// A chunk shown to the model in this example's prompt.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptChunk {
    pub id: i64,
    pub span_range: [i64; 2],
    pub text: String,
}

/// One sentence in the model's fixture response.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseSentence {
    pub text: String,
    pub cited_spans: Vec<i64>,
}

/// An author-supplied support mutation. Hand-crafted contrast-set
/// rewrites of a *valid* example's response sentence. Each mutation
/// names the failure class it's meant to induce (`Unsupported` for
/// off-topic substitutions, `Contradicted` for negation flips / number
/// perturbations / entity swaps).
///
/// The injection harness uses these to derive matched-pair support
/// failures from each valid example (see [`crate::inject`]).
#[derive(Debug, Clone, Deserialize)]
pub struct SupportMutation {
    /// The rewritten sentence text. The cited spans remain the same as
    /// the parent; the change is in the *claim* the citation backs.
    pub text: String,
    /// The class the author intends this mutation to induce.
    /// Must be `Unsupported` or `Contradicted`; the loader rejects
    /// other classes.
    pub class: HallucinationClass,
}

/// One labeled scenario.
#[derive(Debug, Clone, Deserialize)]
pub struct Example {
    pub name: String,
    pub class: HallucinationClass,
    #[serde(default)]
    pub description: String,
    pub corpus_spans: Vec<CorpusSpan>,
    pub prompt_chunks: Vec<PromptChunk>,
    pub response_sentences: Vec<ResponseSentence>,
    /// Author-supplied contrast-set mutations for the support gate.
    /// Meaningful only when `class == Valid` — the injection harness
    /// applies each mutation to the parent's sentence to derive a
    /// matched-pair support failure. Empty by default.
    #[serde(default)]
    pub support_mutations: Vec<SupportMutation>,
}

/// A whole dataset.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dataset {
    #[serde(rename = "example", default)]
    pub examples: Vec<Example>,
}

// Manual Serialize impl skipped: we don't currently round-trip back to
// TOML. Datasets are written by humans and consumed by the runner.

impl Serialize for Example {
    fn serialize<S: serde::Serializer>(&self, _ser: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "Example is read-only; datasets are author-maintained",
        ))
    }
}

/// Load and parse a dataset from a TOML file.
///
/// # Errors
///
/// Returns an error if the file is missing or doesn't parse.
pub fn load_dataset<P: AsRef<Path>>(path: P) -> anyhow::Result<Dataset> {
    let text = std::fs::read_to_string(path.as_ref())?;
    let dataset: Dataset = toml::from_str(&text)?;
    Ok(dataset)
}

/// Load a dataset from either a single TOML file or a directory of them.
///
/// For a directory, every `*.toml` entry (non-recursive) is parsed in
/// lexicographic filename order and their examples are concatenated into
/// one [`Dataset`]. Order is deterministic so reports diff cleanly. This
/// is what lets the 300-example benchmark live as many small files —
/// e.g. one per drug label — while `run` still emits a single matrix.
///
/// # Errors
///
/// Returns an error if `path` doesn't exist, a directory contains no
/// `.toml` files, or any file fails to parse (the error names the file).
pub fn load_dataset_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Dataset> {
    use anyhow::Context as _;

    let path = path.as_ref();
    if !path.is_dir() {
        return load_dataset(path).with_context(|| format!("parsing dataset {}", path.display()));
    }

    let mut toml_files: Vec<std::path::PathBuf> = std::fs::read_dir(path)
        .with_context(|| format!("reading directory {}", path.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    toml_files.sort();

    if toml_files.is_empty() {
        anyhow::bail!("no .toml files in directory {}", path.display());
    }

    let mut examples = Vec::new();
    for file in &toml_files {
        let ds =
            load_dataset(file).with_context(|| format!("parsing dataset {}", file.display()))?;
        examples.extend(ds.examples);
    }
    Ok(Dataset { examples })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_example() {
        let toml_src = r#"
[[example]]
name = "valid_simple"
class = "valid"
description = "A correctly cited, supported sentence."
corpus_spans = [
    { id = 10, page = 1, text = "The contraindication is pregnancy." },
]
prompt_chunks = [
    { id = 1, span_range = [10, 10], text = "The contraindication is pregnancy." },
]
response_sentences = [
    { text = "The contraindication is pregnancy.", cited_spans = [10] },
]
"#;
        let ds: Dataset = toml::from_str(toml_src).unwrap();
        assert_eq!(ds.examples.len(), 1);
        let ex = &ds.examples[0];
        assert_eq!(ex.name, "valid_simple");
        assert_eq!(ex.class, HallucinationClass::Valid);
        assert_eq!(ex.corpus_spans[0].id, 10);
        assert_eq!(ex.prompt_chunks[0].span_range, [10, 10]);
        assert_eq!(ex.response_sentences[0].cited_spans, vec![10]);
    }

    fn write_example(dir: &std::path::Path, file: &str, name: &str) {
        let body = format!(
            r#"
[[example]]
name = "{name}"
class = "valid"
corpus_spans = [{{ id = 10, page = 1, text = "x" }}]
prompt_chunks = [{{ id = 1, span_range = [10, 10], text = "x" }}]
response_sentences = [{{ text = "x", cited_spans = [10] }}]
"#
        );
        std::fs::write(dir.join(file), body).unwrap();
    }

    #[test]
    fn load_path_reads_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("one.toml");
        write_example(dir.path(), "one.toml", "solo");
        let ds = load_dataset_path(&f).unwrap();
        assert_eq!(ds.examples.len(), 1);
        assert_eq!(ds.examples[0].name, "solo");
    }

    #[test]
    fn load_path_merges_directory_in_filename_order() {
        let dir = tempfile::tempdir().unwrap();
        // Intentionally create out of order; loader must sort by name.
        write_example(dir.path(), "b.toml", "second");
        write_example(dir.path(), "a.toml", "first");
        std::fs::write(dir.path().join("notes.md"), "ignored").unwrap();
        let ds = load_dataset_path(dir.path()).unwrap();
        let names: Vec<_> = ds.examples.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["first", "second"]);
    }

    #[test]
    fn load_path_empty_directory_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_dataset_path(dir.path()).unwrap_err();
        assert!(err.to_string().contains("no .toml files"));
    }

    #[test]
    fn load_path_names_the_failing_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.toml"), "this = is not [valid").unwrap();
        let err = load_dataset_path(dir.path()).unwrap_err();
        assert!(err.to_string().contains("bad.toml"));
    }
}
