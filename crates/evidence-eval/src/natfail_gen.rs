//! Natural-failure generation harness.
//!
//! Drives a locally-running Ollama server over a set of (model, PDF,
//! question) tasks and emits a TOML file matching the standard
//! [`crate::Example`] schema — except `class` is left as a comment
//! placeholder for a human annotator. The output is the input to
//! `evidence-eval natfail-prep` after humans fill in the class labels
//! per `docs/paper/natural-failure-protocol.md`.
//!
//! This module *generates* real model answers. It does NOT grade them.
//! The §5.6 result still requires human double-annotation.
//!
//! Networking is isolated behind the [`OllamaClient`] trait so the unit
//! tests can run pure (no localhost dependency).

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Default local Ollama base URL.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// One question to ask each model against one PDF.
#[derive(Debug, Clone, Deserialize)]
pub struct Question {
    /// Filename (basename) of a PDF under the configured `pdf_dir`.
    pub pdf: String,
    /// The natural-language question to put to the model.
    pub prompt: String,
}

/// TOML wrapper for the questions file: `[[question]]` tables.
#[derive(Debug, Clone, Deserialize)]
pub struct QuestionsFile {
    #[serde(default, rename = "question")]
    pub questions: Vec<Question>,
}

/// Configuration for a single `natfail-gen` invocation.
#[derive(Debug, Clone)]
pub struct GenConfig {
    pub pdf_dir: PathBuf,
    pub questions_path: PathBuf,
    pub models: Vec<String>,
    pub output: Option<PathBuf>,
    pub ollama_url: String,
    pub dry_run: bool,
    pub limit: Option<usize>,
}

/// Errors the harness can return.
#[derive(Debug, Error)]
pub enum GenError {
    #[error("Ollama appears to be down at {url}: {message}. Start it with `ollama serve`.")]
    OllamaDown { url: String, message: String },
    #[error("no successful generations — every (model, pdf, question) tuple failed")]
    NoSuccessfulGenerations,
    #[error("failed to read questions file {path}: {source}")]
    ReadQuestions {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse questions TOML {path}: {source}")]
    ParseQuestions {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("questions file {path} declared no [[question]] entries")]
    EmptyQuestions { path: PathBuf },
    #[error("pdf_dir {path} is not a directory")]
    BadPdfDir { path: PathBuf },
    #[error("PDF {path} referenced by question #{index} is missing")]
    MissingPdf { path: PathBuf, index: usize },
    #[error("PDF extraction failed for {path}: {source}")]
    PdfExtract {
        path: PathBuf,
        source: evidence_core::ingest::pdf::PdfError,
    },
    #[error("failed to write output {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// A numbered span shown to a model: a single line from a single page.
#[derive(Debug, Clone)]
pub struct NumberedSpan {
    pub id: i64,
    pub page: u32,
    pub text: String,
}

/// The parsed JSON answer the model is asked to emit.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAnswer {
    pub sentence: String,
    pub cited_spans: Vec<i64>,
}

/// Pluggable Ollama transport. The production impl is
/// [`UreqOllamaClient`]; tests use a stub.
pub trait OllamaClient {
    /// Probe the server. Used to fail fast with a clear message when
    /// Ollama isn't running.
    fn ping(&self) -> Result<(), String>;
    /// Send a single non-streaming `/api/generate` request. Returns the
    /// raw `response` field on success, or an [`OllamaCallError`]
    /// distinguishing "model not found" from generic transport errors.
    fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaCallError>;
}

/// Error returned by [`OllamaClient::generate`]. The harness treats
/// `ModelMissing` as "skip + log" and other variants as per-task
/// failures.
#[derive(Debug)]
pub enum OllamaCallError {
    ModelMissing(String),
    Other(String),
}

/// `ureq`-backed Ollama client. Talks to `<base_url>/api/generate`
/// (non-streaming, `format: "json"`) and `<base_url>/api/tags` for the
/// liveness probe.
pub struct UreqOllamaClient {
    base_url: String,
    agent: ureq::Agent,
}

impl UreqOllamaClient {
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(120))
                .build(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

impl OllamaClient for UreqOllamaClient {
    fn ping(&self) -> Result<(), String> {
        self.agent
            .get(&self.url("/api/tags"))
            .call()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaCallError> {
        let body = serde_json::json!({
            "model": model,
            "stream": false,
            "format": "json",
            "prompt": prompt,
        });
        let resp = self
            .agent
            .post(&self.url("/api/generate"))
            .send_json(body)
            .map_err(|e| {
                let s = e.to_string();
                // Ollama returns HTTP 404 with a body like
                // {"error":"model 'foo' not found, try pulling it first"}
                // when a model isn't pulled. ureq surfaces the body in
                // Status errors; sniff for the standard phrase.
                if s.contains("not found") || s.contains("404") {
                    OllamaCallError::ModelMissing(s)
                } else {
                    OllamaCallError::Other(s)
                }
            })?;
        #[derive(Deserialize)]
        struct R {
            response: String,
        }
        let payload: R = resp
            .into_json()
            .map_err(|e| OllamaCallError::Other(e.to_string()))?;
        Ok(payload.response)
    }
}

/// Extract spans from a PDF and number them globally across pages,
/// starting at 1. Lightweight wrapper so the natfail-gen schema stays
/// independent of the underlying PDF type.
pub fn number_spans(pdf: &Path) -> Result<Vec<NumberedSpan>, GenError> {
    let pages =
        evidence_core::ingest::pdf::extract(pdf).map_err(|source| GenError::PdfExtract {
            path: pdf.to_path_buf(),
            source,
        })?;
    let mut out = Vec::new();
    let mut next_id: i64 = 1;
    for page in pages {
        for span in page.spans {
            let text = span.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            out.push(NumberedSpan {
                id: next_id,
                page: page.page_num,
                text,
            });
            next_id += 1;
        }
    }
    Ok(out)
}

/// Build the full prompt sent to a model: instructions, the numbered
/// spans, and the question. The instructions ask for a single sentence
/// with citations to numbered span IDs, in JSON.
#[must_use]
pub fn build_prompt(spans: &[NumberedSpan], question: &str) -> String {
    let mut s = String::with_capacity(2048);
    s.push_str(
        "You are answering a factual question using ONLY the numbered spans below.\n\
         Return a SINGLE JSON object with this exact schema:\n\
         {\"sentence\": \"<one sentence answer>\", \"cited_spans\": [<int>, ...]}\n\
         Rules:\n\
         - Cite ONLY span IDs that appear below. Do not invent IDs.\n\
         - Every claim in the sentence must be supported by the cited spans.\n\
         - If the spans do not answer the question, return an empty cited_spans array.\n\n",
    );
    s.push_str("Numbered spans:\n");
    for sp in spans {
        s.push_str(&format!("[{}] (p.{}) {}\n", sp.id, sp.page, sp.text));
    }
    s.push_str("\nQuestion: ");
    s.push_str(question);
    s.push('\n');
    s
}

/// Best-effort parse of a model's JSON answer.
///
/// Accepts:
/// - A pure JSON object: `{"sentence": "...", "cited_spans": [1, 2]}`
/// - A JSON object embedded in prose (e.g. "Here you go: { ... } done.")
/// - String `cited_spans` entries that are integers in disguise
///   (`"3"` -> `3`).
///
/// Rejects: anything where we cannot find a balanced top-level object
/// containing both `sentence` and `cited_spans`.
pub fn parse_answer(raw: &str) -> Result<ParsedAnswer, String> {
    let slice = extract_first_json_object(raw).ok_or_else(|| "no JSON object found".to_string())?;
    #[derive(Deserialize)]
    struct Flexible {
        #[serde(default)]
        sentence: Option<String>,
        #[serde(default)]
        cited_spans: Option<serde_json::Value>,
    }
    let parsed: Flexible =
        serde_json::from_str(slice).map_err(|e| format!("JSON parse failed: {e}"))?;
    let sentence = parsed
        .sentence
        .ok_or_else(|| "missing `sentence` field".to_string())?
        .trim()
        .to_string();
    if sentence.is_empty() {
        return Err("`sentence` is empty".to_string());
    }
    let cited_spans = match parsed.cited_spans {
        None => Vec::new(),
        Some(serde_json::Value::Null) => Vec::new(),
        Some(serde_json::Value::Array(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            out.push(i);
                        }
                    }
                    serde_json::Value::String(s) => {
                        if let Ok(i) = s.trim().parse::<i64>() {
                            out.push(i);
                        }
                    }
                    _ => {}
                }
            }
            out
        }
        _ => return Err("`cited_spans` is not an array".to_string()),
    };
    Ok(ParsedAnswer {
        sentence,
        cited_spans,
    })
}

/// Find the first balanced `{...}` substring in `raw`. Skips object
/// braces inside JSON strings. Returns `None` if no balanced object is
/// present.
fn extract_first_json_object(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let mut start: Option<usize> = None;
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => {
                if start.is_none() {
                    start = Some(i);
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        return Some(&raw[s..=i]);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Identifier for one planned task (one row of the cartesian product).
#[derive(Debug, Clone)]
pub struct PlannedTask {
    pub model: String,
    pub pdf: PathBuf,
    pub pdf_short: String,
    pub question_index: usize,
    pub question: String,
}

/// Build the planned (model, pdf, question) tuples. Pure — no IO beyond
/// existence checks on the PDF files.
pub fn plan_tasks(
    config: &GenConfig,
    questions: &[Question],
) -> Result<Vec<PlannedTask>, GenError> {
    if !config.pdf_dir.is_dir() {
        return Err(GenError::BadPdfDir {
            path: config.pdf_dir.clone(),
        });
    }
    let mut out = Vec::new();
    for model in &config.models {
        for (qi, q) in questions.iter().enumerate() {
            let pdf = config.pdf_dir.join(&q.pdf);
            if !pdf.is_file() {
                return Err(GenError::MissingPdf {
                    path: pdf,
                    index: qi,
                });
            }
            let pdf_short = pdf_short_name(&q.pdf);
            out.push(PlannedTask {
                model: model.clone(),
                pdf,
                pdf_short,
                question_index: qi,
                question: q.prompt.clone(),
            });
        }
    }
    if let Some(limit) = config.limit {
        out.truncate(limit);
    }
    Ok(out)
}

/// Sanitize a PDF basename into the slug component used in
/// `natfail_<model>_<pdfshort>_<qN>`.
#[must_use]
pub fn pdf_short_name(pdf: &str) -> String {
    let stem = std::path::Path::new(pdf)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(pdf);
    sanitize_slug(stem)
}

/// Replace anything that isn't `[A-Za-z0-9_]` with `_`. Used for every
/// component of an example name so the output is grep-friendly.
#[must_use]
pub fn sanitize_slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('x');
    }
    out
}

/// One generated example, ready to be rendered to TOML.
#[derive(Debug)]
pub struct GeneratedExample {
    pub name: String,
    pub description: String,
    pub spans: Vec<NumberedSpan>,
    pub chunk_text: String,
    pub answer: ParsedAnswer,
}

/// Render the planned tasks as a TOML preview (what the model *would* be
/// sent), with no network calls. Used by `--dry-run`.
pub fn render_dry_run_plan(tasks: &[PlannedTask]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "# natfail-gen dry run: {} planned tasks", tasks.len());
    for t in tasks {
        let _ = writeln!(
            out,
            "# - model={}, pdf={}, q#{}: {}",
            t.model,
            t.pdf.display(),
            t.question_index,
            one_line(&t.question)
        );
    }
    out
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

/// Render a list of generated examples to the standard `[[example]]`
/// TOML schema. The `class` field is intentionally written as a TOML
/// comment placeholder so the file is not silently deserializable —
/// natfail-prep will reject it until a human fills the class.
#[must_use]
pub fn render_examples_toml(examples: &[GeneratedExample]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("# Auto-generated by `evidence-eval natfail-gen`. Each [[example]] is a\n");
    out.push_str("# REAL model answer. Replace the `# class = ...` placeholder with one of:\n");
    out.push_str(
        "#   valid | uncited | fabricated_span | out_of_context | unsupported | contradicted\n",
    );
    out.push_str("# (or extend the taxonomy per natural-failure-protocol.md), then pass the\n");
    out.push_str("# file to `evidence-eval natfail-prep`.\n\n");
    for ex in examples {
        let _ = writeln!(out, "[[example]]");
        let _ = writeln!(out, "name = {}", toml_str(&ex.name));
        out.push_str("# class = \"FILL_IN_HUMAN_LABEL\"\n");
        if !ex.description.is_empty() {
            let _ = writeln!(out, "description = {}", toml_str(&ex.description));
        }
        out.push_str("corpus_spans = [\n");
        for s in &ex.spans {
            let _ = writeln!(
                out,
                "    {{ id = {}, page = {}, text = {} }},",
                s.id,
                s.page,
                toml_str(&s.text)
            );
        }
        out.push_str("]\n");
        let max_id = ex.spans.last().map(|s| s.id).unwrap_or(0);
        let min_id = ex.spans.first().map(|s| s.id).unwrap_or(0);
        out.push_str("prompt_chunks = [\n");
        let _ = writeln!(
            out,
            "    {{ id = 1, span_range = [{}, {}], text = {} }},",
            min_id,
            max_id,
            toml_str(&ex.chunk_text)
        );
        out.push_str("]\n");
        out.push_str("response_sentences = [\n");
        let cites = ex
            .answer
            .cited_spans
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "    {{ text = {}, cited_spans = [{}] }},",
            toml_str(&ex.answer.sentence),
            cites
        );
        out.push_str("]\n\n");
    }
    out
}

/// TOML basic-string with quote/backslash escapes. Newlines become `\n`.
fn toml_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Build the chunk text shown to the model: one line per numbered span.
#[must_use]
pub fn build_chunk_text(spans: &[NumberedSpan]) -> String {
    let mut out = String::new();
    for s in spans {
        out.push_str(&format!("[{}] (p.{}) {}\n", s.id, s.page, s.text));
    }
    out
}

/// Result of a [`run`] invocation. The caller decides exit codes from
/// the summary.
#[derive(Debug, Default)]
pub struct GenSummary {
    pub ok: usize,
    pub skipped_model_missing: usize,
    pub skipped_parse: usize,
    pub skipped_transport: usize,
    pub skipped_pdf: usize,
    pub total: usize,
    pub output_path: Option<PathBuf>,
}

/// Run the harness. Reads PDFs, drives the [`OllamaClient`], parses
/// answers, and writes the output TOML.
///
/// `--dry-run` skips the client entirely and writes a plan preview to
/// the same output sink. Returns a [`GenSummary`] the caller can use to
/// pick an exit code.
pub fn run<C: OllamaClient>(
    config: &GenConfig,
    client: &C,
    log: &mut dyn std::io::Write,
) -> Result<GenSummary, GenError> {
    let questions_text = std::fs::read_to_string(&config.questions_path).map_err(|source| {
        GenError::ReadQuestions {
            path: config.questions_path.clone(),
            source,
        }
    })?;
    let qf: QuestionsFile =
        toml::from_str(&questions_text).map_err(|source| GenError::ParseQuestions {
            path: config.questions_path.clone(),
            source,
        })?;
    if qf.questions.is_empty() {
        return Err(GenError::EmptyQuestions {
            path: config.questions_path.clone(),
        });
    }
    let tasks = plan_tasks(config, &qf.questions)?;

    if config.dry_run {
        let plan = render_dry_run_plan(&tasks);
        let summary = write_output(config, &plan)?;
        return Ok(GenSummary {
            total: tasks.len(),
            output_path: summary,
            ..GenSummary::default()
        });
    }

    // Real run: ping first so a downed Ollama gives one clean error.
    client.ping().map_err(|message| GenError::OllamaDown {
        url: config.ollama_url.clone(),
        message,
    })?;

    let mut examples: Vec<GeneratedExample> = Vec::new();
    let mut summary = GenSummary {
        total: tasks.len(),
        ..GenSummary::default()
    };
    // Cache PDF extraction per file across models.
    let mut span_cache: std::collections::HashMap<PathBuf, Vec<NumberedSpan>> =
        std::collections::HashMap::new();

    for task in &tasks {
        let spans = match span_cache.get(&task.pdf) {
            Some(s) => s.clone(),
            None => match number_spans(&task.pdf) {
                Ok(s) => {
                    span_cache.insert(task.pdf.clone(), s.clone());
                    s
                }
                Err(e) => {
                    let _ = writeln!(
                        log,
                        "skip: pdf extract failed for {}: {e}",
                        task.pdf.display()
                    );
                    summary.skipped_pdf += 1;
                    continue;
                }
            },
        };
        let prompt = build_prompt(&spans, &task.question);
        let raw = match client.generate(&task.model, &prompt) {
            Ok(s) => s,
            Err(OllamaCallError::ModelMissing(msg)) => {
                let _ = writeln!(
                    log,
                    "skip: model {} not pulled ({msg}); `ollama pull {}` to enable",
                    task.model, task.model,
                );
                summary.skipped_model_missing += 1;
                continue;
            }
            Err(OllamaCallError::Other(msg)) => {
                let _ = writeln!(
                    log,
                    "skip: transport error for model={} pdf={} q#{}: {msg}",
                    task.model,
                    task.pdf.display(),
                    task.question_index,
                );
                summary.skipped_transport += 1;
                continue;
            }
        };
        let answer = match parse_answer(&raw) {
            Ok(a) => a,
            Err(e) => {
                let _ = writeln!(
                    log,
                    "skip: parse failure for model={} pdf={} q#{}: {e}",
                    task.model,
                    task.pdf.display(),
                    task.question_index,
                );
                summary.skipped_parse += 1;
                continue;
            }
        };
        let name = format!(
            "natfail_{}_{}_q{}",
            sanitize_slug(&task.model),
            task.pdf_short,
            task.question_index + 1,
        );
        let description = format!(
            "model={} pdf={} question={}",
            task.model,
            task.pdf
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("<unknown>"),
            one_line(&task.question),
        );
        let chunk_text = build_chunk_text(&spans);
        examples.push(GeneratedExample {
            name,
            description,
            spans,
            chunk_text,
            answer,
        });
        summary.ok += 1;
    }

    if summary.ok == 0 {
        let _ = writeln!(log, "no successful generations: 0 / {}", summary.total);
        return Err(GenError::NoSuccessfulGenerations);
    }

    let rendered = render_examples_toml(&examples);
    let out_path = write_output(config, &rendered)?;
    summary.output_path = out_path;
    Ok(summary)
}

fn write_output(config: &GenConfig, body: &str) -> Result<Option<PathBuf>, GenError> {
    match &config.output {
        Some(p) => {
            std::fs::write(p, body.as_bytes()).map_err(|source| GenError::Write {
                path: p.clone(),
                source,
            })?;
            Ok(Some(p.clone()))
        }
        None => {
            print!("{body}");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_json() {
        let raw =
            r#"{"sentence": "Aspirin is contraindicated in pregnancy.", "cited_spans": [3, 7]}"#;
        let p = parse_answer(raw).unwrap();
        assert_eq!(p.sentence, "Aspirin is contraindicated in pregnancy.");
        assert_eq!(p.cited_spans, vec![3, 7]);
    }

    #[test]
    fn parses_json_embedded_in_prose() {
        let raw = r#"Sure! Here is the answer:
        {"sentence": "Foo bar.", "cited_spans": [1]}
        Hope that helps."#;
        let p = parse_answer(raw).unwrap();
        assert_eq!(p.sentence, "Foo bar.");
        assert_eq!(p.cited_spans, vec![1]);
    }

    #[test]
    fn parses_string_span_ids() {
        let raw = r#"{"sentence": "x", "cited_spans": ["3", "9"]}"#;
        let p = parse_answer(raw).unwrap();
        assert_eq!(p.cited_spans, vec![3, 9]);
    }

    #[test]
    fn parses_empty_cited_spans() {
        let raw = r#"{"sentence": "x", "cited_spans": []}"#;
        let p = parse_answer(raw).unwrap();
        assert!(p.cited_spans.is_empty());
    }

    #[test]
    fn parses_missing_cited_spans_as_empty() {
        let raw = r#"{"sentence": "x"}"#;
        let p = parse_answer(raw).unwrap();
        assert!(p.cited_spans.is_empty());
    }

    #[test]
    fn rejects_when_no_object_present() {
        assert!(parse_answer("nope").is_err());
    }

    #[test]
    fn rejects_empty_sentence() {
        let raw = r#"{"sentence": "", "cited_spans": [1]}"#;
        assert!(parse_answer(raw).is_err());
    }

    #[test]
    fn extracts_first_balanced_object_with_brace_in_string() {
        let raw = r#"prefix {"sentence": "has a } in it", "cited_spans": [2]} suffix"#;
        let p = parse_answer(raw).unwrap();
        assert_eq!(p.sentence, "has a } in it");
        assert_eq!(p.cited_spans, vec![2]);
    }

    #[test]
    fn renders_examples_toml_uses_comment_for_class() {
        let ex = GeneratedExample {
            name: "natfail_llama_doc_q1".to_string(),
            description: "model=llama pdf=doc.pdf question=Why?".to_string(),
            spans: vec![
                NumberedSpan {
                    id: 1,
                    page: 1,
                    text: "First span.".to_string(),
                },
                NumberedSpan {
                    id: 2,
                    page: 1,
                    text: "Second span.".to_string(),
                },
            ],
            chunk_text: "[1] (p.1) First span.\n[2] (p.1) Second span.\n".to_string(),
            answer: ParsedAnswer {
                sentence: "It is so.".to_string(),
                cited_spans: vec![2],
            },
        };
        let text = render_examples_toml(&[ex]);
        assert!(text.contains("[[example]]"));
        assert!(text.contains("name = \"natfail_llama_doc_q1\""));
        assert!(text.contains("# class = "));
        assert!(!text.contains("\nclass = "));
        assert!(text.contains("First span."));
        assert!(text.contains("cited_spans = [2]"));
    }

    #[test]
    fn sanitize_slug_replaces_special_chars() {
        assert_eq!(sanitize_slug("llama3.1:8b"), "llama3_1_8b");
        assert_eq!(sanitize_slug("plain"), "plain");
        assert_eq!(sanitize_slug(""), "x");
    }

    #[test]
    fn pdf_short_strips_extension() {
        assert_eq!(pdf_short_name("foo-bar.pdf"), "foo_bar");
    }

    #[test]
    fn build_prompt_includes_spans_and_question() {
        let spans = vec![NumberedSpan {
            id: 5,
            page: 2,
            text: "Avoid in pregnancy.".to_string(),
        }];
        let p = build_prompt(&spans, "When to avoid?");
        assert!(p.contains("[5] (p.2) Avoid in pregnancy."));
        assert!(p.contains("Question: When to avoid?"));
        assert!(p.contains("Return a SINGLE JSON object"));
    }

    #[test]
    fn dry_run_plan_lists_tasks() {
        let tasks = vec![
            PlannedTask {
                model: "llama".to_string(),
                pdf: PathBuf::from("/tmp/a.pdf"),
                pdf_short: "a".to_string(),
                question_index: 0,
                question: "Why?".to_string(),
            },
            PlannedTask {
                model: "qwen".to_string(),
                pdf: PathBuf::from("/tmp/a.pdf"),
                pdf_short: "a".to_string(),
                question_index: 0,
                question: "Why?".to_string(),
            },
        ];
        let plan = render_dry_run_plan(&tasks);
        assert!(plan.contains("2 planned tasks"));
        assert!(plan.contains("model=llama"));
        assert!(plan.contains("model=qwen"));
    }

    /// Stub client used by the dry-run test below.
    struct StubClient {
        ping_ok: bool,
    }

    impl OllamaClient for StubClient {
        fn ping(&self) -> Result<(), String> {
            if self.ping_ok {
                Ok(())
            } else {
                Err("connection refused".to_string())
            }
        }

        fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaCallError> {
            Err(OllamaCallError::Other("unused in dry-run".to_string()))
        }
    }

    #[test]
    fn dry_run_writes_plan_without_calling_client() {
        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("doc.pdf");
        std::fs::write(&pdf_path, b"%PDF-1.4\n").unwrap();
        let q_path = dir.path().join("q.toml");
        std::fs::write(
            &q_path,
            "[[question]]\npdf = \"doc.pdf\"\nprompt = \"What is the contraindication?\"\n",
        )
        .unwrap();
        let out_path = dir.path().join("out.toml");
        let config = GenConfig {
            pdf_dir: dir.path().to_path_buf(),
            questions_path: q_path,
            models: vec!["llama".to_string(), "qwen".to_string()],
            output: Some(out_path.clone()),
            ollama_url: DEFAULT_OLLAMA_URL.to_string(),
            dry_run: true,
            limit: None,
        };
        let client = StubClient { ping_ok: false };
        let mut log = Vec::new();
        let summary = run(&config, &client, &mut log).unwrap();
        assert_eq!(summary.total, 2);
        let text = std::fs::read_to_string(&out_path).unwrap();
        assert!(text.contains("2 planned tasks"));
        assert!(text.contains("model=llama"));
    }
}
