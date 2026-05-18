//! `evidence-eval` — run a TOML eval dataset through every
//! `ValidationPolicy` and produce a markdown report, or generate
//! injected failure variants from a dataset of valid seeds.
//!
//! ```sh
//! # Run the validator harness on a dataset
//! evidence-eval run crates/evidence-eval/datasets/bootstrap.toml
//!
//! # Save the report; non-zero exit if any row disagrees
//! evidence-eval run crates/evidence-eval/datasets/bootstrap.toml --output report.md
//!
//! # Run with the real NLI cross-encoder (downloads model on first call)
//! evidence-eval run crates/evidence-eval/datasets/bootstrap.toml --real-nli
//!
//! # Generate matched-pair failure variants from a dataset of valid seeds
//! evidence-eval inject crates/evidence-eval/datasets/valid_seeds.toml \
//!     --output crates/evidence-eval/datasets/injected.toml
//! ```

#![deny(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use evidence_eval::{
    audit_report, compare_report, fetch_dailymed, has_errors, has_missing, inject_all_variants,
    lint, load_dataset, load_dataset_path, normalize, render_for_pdf, render_report, render_stats,
    rule_metrics, run_with_model, write_markdown, AuthorOptions, DatasetStats, FetchConfig,
    InjectionConfig, Report, SupportMode, UreqClient,
};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "evidence-eval",
    version,
    about = "Closed-Loop Citation eval harness"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the validator harness on a labeled dataset.
    Run {
        /// Path to a TOML eval dataset, or a directory of `*.toml`
        /// datasets (merged in filename order into one report).
        dataset: PathBuf,
        /// Optional output path for the markdown report. Default: stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Use the real NLI cross-encoder for the support gate. First
        /// run downloads `Xenova/distilbert-base-uncased-mnli` (~265 MB).
        #[arg(long)]
        real_nli: bool,
        /// Override the NLI model repo (HuggingFace id) used by
        /// `--real-nli`. Lets you measure a stronger checkpoint (e.g. a
        /// DeBERTa-v3-large MNLI model) on the same labeled set for the
        /// paper's model comparison. Ignored without `--real-nli`. The
        /// repo must expose `onnx/model.onnx`, `tokenizer.json`, and a
        /// `config.json` with an `id2label` MNLI permutation.
        #[arg(long, value_name = "HF_REPO")]
        nli_model: Option<String>,
    },
    /// Run a dataset under two NLI models (both real-NLI) and emit the
    /// §5.1 model-comparison table: per-class three-gate agreement for
    /// each model plus the delta. The headline is valid-retention — does
    /// the candidate stop false-refusing true claims? Both models
    /// download on first use.
    Compare {
        /// Path to a TOML eval dataset, or a directory of `*.toml`.
        dataset: PathBuf,
        /// Candidate NLI repo (HuggingFace id) to compare against the
        /// baseline — e.g. a DeBERTa-v3-large MNLI checkpoint.
        #[arg(long, value_name = "HF_REPO")]
        candidate_nli: String,
        /// Baseline NLI repo. Default: the built-in distilbert.
        #[arg(long, value_name = "HF_REPO")]
        baseline_nli: Option<String>,
        /// Optional output path for the comparison markdown. Default: stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Compute §5.2 per-rule precision/recall/F1 (marginal-gate
    /// decomposition) and §5.3 non-overlap evidence for a dataset.
    /// Structural rules are model-independent; pass `--real-nli`
    /// (optionally `--nli-model`) so the support row reflects a real
    /// checkpoint instead of the deterministic mock.
    Metrics {
        /// Path to a TOML eval dataset, or a directory of `*.toml`.
        /// Inject first — metrics needs the failure variants.
        dataset: PathBuf,
        /// Use the real NLI cross-encoder for the support row.
        #[arg(long)]
        real_nli: bool,
        /// Override the NLI model repo (HF id) used by `--real-nli`.
        #[arg(long, value_name = "HF_REPO")]
        nli_model: Option<String>,
        /// Optional output path for the metrics markdown. Default: stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate matched-pair failure variants from a dataset of valid
    /// seeds. Each valid example yields two structural injections
    /// (existence + in-context) plus one variant per `support_mutations`
    /// entry.
    Inject {
        /// Input TOML containing valid seed examples.
        input: PathBuf,
        /// Output TOML to write. The output contains the seeds plus the
        /// generated variants.
        #[arg(long)]
        output: PathBuf,
    },
    /// Fetch a public corpus into a local directory. Currently supports
    /// only DailyMed (FDA drug labels). Emits `manifest.toml` plus the
    /// PDFs under `labels/`.
    Fetch {
        #[command(subcommand)]
        source: FetchSource,
    },
    /// Lint a dataset for authoring mistakes. Exits 2 on any error,
    /// 0 on warnings-only or clean. Run before `run` / `inject` to
    /// catch wrong-class structure, fabricated spans that aren't
    /// fabricated, etc.
    Lint {
        /// Path to a TOML eval dataset, or a directory of `*.toml`
        /// datasets (linted together as one set).
        dataset: PathBuf,
    },
    /// Audit dataset faithfulness: verify every cited corpus span is
    /// real source text in the corpus PDFs. Catches hallucinated /
    /// invented span text — the primary reviewer attack on an
    /// AI-authored benchmark. Exits 2 if any span is Missing (not found
    /// verbatim or hyphen-insensitively), 0 otherwise.
    AuditFaithfulness {
        /// Path to a TOML eval dataset, or a directory of `*.toml`.
        #[arg(long)]
        dataset: PathBuf,
        /// Corpus directory. PDFs are read from `<corpus>/labels/*.pdf`
        /// (falling back to `<corpus>/*.pdf` if `labels/` has none).
        #[arg(long)]
        corpus: PathBuf,
    },
    /// Print descriptive stats for a dataset: class balance, per-example
    /// shape, and what `inject` will materialize. Read-only; never fails
    /// on a parseable dataset.
    Stats {
        /// Path to a TOML eval dataset, or a directory of `*.toml`
        /// datasets (aggregated into one summary).
        dataset: PathBuf,
    },
    /// Render a TOML authoring skeleton from a PDF. Lists every span
    /// (with assigned IDs) as a comment block, followed by an empty
    /// `[[example]]` block to fill in. Pipe to a file, edit, and run
    /// `evidence-eval lint` before `run`.
    Author {
        /// Path to the source PDF.
        #[arg(long)]
        pdf: PathBuf,
        /// Only list spans on this 1-indexed page. Omit to list all.
        #[arg(long)]
        page: Option<u32>,
        /// Suggested `name` for the skeleton `[[example]]` block.
        #[arg(long, default_value = "FILL_IN_AUTHOR_NAME")]
        name: String,
        /// Suggested `class` for the skeleton `[[example]]` block.
        #[arg(long, default_value = "valid")]
        class: String,
    },
}

#[derive(Subcommand)]
enum FetchSource {
    /// DailyMed — FDA Structured Product Labeling. Public domain.
    Dailymed {
        /// Output directory for `manifest.toml` and `labels/*.pdf`.
        #[arg(long, default_value = "dailymed-corpus")]
        output: PathBuf,
        /// Maximum number of labels to fetch.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Pause between HTTP requests, in milliseconds.
        #[arg(long, default_value_t = 500)]
        delay_ms: u64,
    },
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            dataset,
            output,
            real_nli,
            nli_model,
        } => run_command(&dataset, output.as_deref(), real_nli, nli_model.as_deref()),
        Command::Compare {
            dataset,
            candidate_nli,
            baseline_nli,
            output,
        } => compare_command(
            &dataset,
            &candidate_nli,
            baseline_nli.as_deref(),
            output.as_deref(),
        ),
        Command::Metrics {
            dataset,
            real_nli,
            nli_model,
            output,
        } => metrics_command(&dataset, real_nli, nli_model.as_deref(), output.as_deref()),
        Command::Inject { input, output } => inject_command(&input, &output),
        Command::Fetch { source } => match source {
            FetchSource::Dailymed {
                output,
                limit,
                delay_ms,
            } => fetch_dailymed_command(&output, limit, delay_ms),
        },
        Command::AuditFaithfulness { dataset, corpus } => {
            audit_faithfulness_command(&dataset, &corpus)
        }
        Command::Lint { dataset } => lint_command(&dataset),
        Command::Stats { dataset } => stats_command(&dataset),
        Command::Author {
            pdf,
            page,
            name,
            class,
        } => author_command(&pdf, page, name, class),
    }
}

fn stats_command(dataset_path: &std::path::Path) -> Result<ExitCode> {
    let dataset = load_dataset_path(dataset_path).context("loading dataset for stats")?;
    let stats = DatasetStats::compute(&dataset);
    print!("{}", render_stats(&stats));
    Ok(ExitCode::SUCCESS)
}

fn author_command(
    pdf: &std::path::Path,
    page: Option<u32>,
    name: String,
    class: String,
) -> Result<ExitCode> {
    let options = AuthorOptions {
        page,
        name,
        class,
        ..AuthorOptions::default()
    };
    let rendered = render_for_pdf(pdf, &options).context("rendering author template")?;
    print!("{rendered}");
    Ok(ExitCode::SUCCESS)
}

fn audit_faithfulness_command(
    dataset_path: &std::path::Path,
    corpus_dir: &std::path::Path,
) -> Result<ExitCode> {
    let dataset = load_dataset_path(dataset_path).context("loading dataset for audit")?;

    let labels_dir = corpus_dir.join("labels");
    let mut pdfs = collect_pdfs(&labels_dir)?;
    if pdfs.is_empty() {
        pdfs = collect_pdfs(corpus_dir)?;
    }
    if pdfs.is_empty() {
        anyhow::bail!(
            "no *.pdf files found under {} or {}",
            labels_dir.display(),
            corpus_dir.display()
        );
    }
    pdfs.sort();

    let mut raw = String::new();
    for pdf in &pdfs {
        let pages = evidence_core::ingest::pdf::extract(pdf)
            .with_context(|| format!("extracting {}", pdf.display()))?;
        for page in &pages {
            raw.push_str(&page.raw_text);
            raw.push(' ');
        }
    }
    let corpus_norm = normalize(&raw);

    print!("{}", audit_report(&dataset, &corpus_norm));

    if has_missing(&dataset, &corpus_norm) {
        Ok(ExitCode::from(2))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Collect every `*.pdf` directly under `dir` (non-recursive). Returns an
/// empty vec if `dir` is missing or not a directory.
fn collect_pdfs(dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            out.push(path);
        }
    }
    Ok(out)
}

fn lint_command(dataset_path: &std::path::Path) -> Result<ExitCode> {
    let dataset = load_dataset_path(dataset_path).context("loading dataset for lint")?;
    let diags = lint(&dataset);
    let report = render_report(&diags);
    print!("{report}");
    if has_errors(&diags) {
        Ok(ExitCode::from(2))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn fetch_dailymed_command(
    output: &std::path::Path,
    limit: usize,
    delay_ms: u64,
) -> Result<ExitCode> {
    let http = UreqClient::new();
    let config = FetchConfig {
        limit,
        delay: Duration::from_millis(delay_ms),
        output_dir: output.to_path_buf(),
    };
    let manifest = fetch_dailymed(&http, &config).context("fetching DailyMed corpus")?;
    eprintln!(
        "fetched {} labels into {}",
        manifest.count,
        output.display(),
    );
    Ok(ExitCode::SUCCESS)
}

fn run_command(
    dataset_path: &std::path::Path,
    output: Option<&std::path::Path>,
    real_nli: bool,
    nli_model: Option<&str>,
) -> Result<ExitCode> {
    let dataset = load_dataset_path(dataset_path).context("loading dataset")?;
    let mode = if real_nli {
        SupportMode::RealNli
    } else {
        SupportMode::Mock
    };
    if nli_model.is_some() && !real_nli {
        eprintln!("warning: --nli-model is ignored without --real-nli");
    }
    let rows = run_with_model(&dataset, mode, nli_model).context("running eval")?;
    let report = Report::new(rows);
    let md = write_markdown(&report);

    if let Some(out_path) = output {
        std::fs::write(out_path, md.as_bytes()).context("writing report")?;
        eprintln!("wrote report to {}", out_path.display());
    } else {
        print!("{md}");
    }

    let (agree, total) = report.agreement();
    if agree == total {
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!("disagreement: {agree} / {total} rows match expected matrix");
        Ok(ExitCode::from(2))
    }
}

fn compare_command(
    dataset_path: &std::path::Path,
    candidate_nli: &str,
    baseline_nli: Option<&str>,
    output: Option<&std::path::Path>,
) -> Result<ExitCode> {
    let dataset = load_dataset_path(dataset_path).context("loading dataset")?;
    let baseline_rows =
        run_with_model(&dataset, SupportMode::RealNli, baseline_nli).context("baseline run")?;
    let candidate_rows = run_with_model(&dataset, SupportMode::RealNli, Some(candidate_nli))
        .context("candidate run")?;
    let baseline_label = baseline_nli.unwrap_or("distilbert (default)");
    let md = compare_report(
        baseline_label,
        &baseline_rows,
        candidate_nli,
        &candidate_rows,
    );
    if let Some(out_path) = output {
        std::fs::write(out_path, md.as_bytes()).context("writing comparison")?;
        eprintln!("wrote comparison to {}", out_path.display());
    } else {
        print!("{md}");
    }
    Ok(ExitCode::SUCCESS)
}

fn metrics_command(
    dataset_path: &std::path::Path,
    real_nli: bool,
    nli_model: Option<&str>,
    output: Option<&std::path::Path>,
) -> Result<ExitCode> {
    let dataset = load_dataset_path(dataset_path).context("loading dataset")?;
    let mode = if real_nli {
        SupportMode::RealNli
    } else {
        SupportMode::Mock
    };
    if nli_model.is_some() && !real_nli {
        eprintln!("warning: --nli-model is ignored without --real-nli");
    }
    let rows = run_with_model(&dataset, mode, nli_model).context("running eval")?;
    let md = rule_metrics(&rows);
    if let Some(out_path) = output {
        std::fs::write(out_path, md.as_bytes()).context("writing metrics")?;
        eprintln!("wrote metrics to {}", out_path.display());
    } else {
        print!("{md}");
    }
    Ok(ExitCode::SUCCESS)
}

fn inject_command(input: &std::path::Path, output: &std::path::Path) -> Result<ExitCode> {
    let dataset = load_dataset(input).context("loading seed dataset")?;
    let config = InjectionConfig::default();

    let mut injected = Vec::new();
    let mut skipped = 0usize;
    for example in &dataset.examples {
        if example.class != evidence_eval::HallucinationClass::Valid {
            skipped += 1;
            continue;
        }
        let variants = inject_all_variants(example, &config)
            .with_context(|| format!("injecting variants of {}", example.name))?;
        injected.extend(variants);
    }

    let total = dataset.examples.len() + injected.len();
    let toml_text = render_dataset_as_toml(&dataset, &injected)?;
    std::fs::write(output, toml_text.as_bytes()).context("writing injected dataset")?;
    eprintln!(
        "wrote {total} examples to {} ({} seeds + {} injected; {skipped} non-valid skipped)",
        output.display(),
        dataset.examples.len(),
        injected.len(),
    );
    Ok(ExitCode::SUCCESS)
}

/// Hand-emit TOML for the augmented dataset. Authored datasets keep
/// formatting + comments; this writes a clean, machine-emitted file
/// suitable for piping into `evidence-eval run`.
fn render_dataset_as_toml(
    seeds: &evidence_eval::Dataset,
    injected: &[evidence_eval::Example],
) -> Result<String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("# Auto-generated by `evidence-eval inject`. Do not edit by hand —\n");
    out.push_str("# re-run `evidence-eval inject` to refresh from the seed file.\n\n");
    for example in seeds.examples.iter().chain(injected.iter()) {
        let _ = writeln!(out, "[[example]]");
        let _ = writeln!(out, "name = {}", toml_str(&example.name));
        let _ = writeln!(out, "class = {}", toml_str(class_label(example.class)));
        if !example.description.is_empty() {
            let _ = writeln!(out, "description = {}", toml_str(&example.description));
        }
        out.push_str("corpus_spans = [\n");
        for s in &example.corpus_spans {
            let _ = writeln!(
                out,
                "    {{ id = {}, page = {}, text = {} }},",
                s.id,
                s.page,
                toml_str(&s.text)
            );
        }
        out.push_str("]\n");
        out.push_str("prompt_chunks = [\n");
        for c in &example.prompt_chunks {
            let _ = writeln!(
                out,
                "    {{ id = {}, span_range = [{}, {}], text = {} }},",
                c.id,
                c.span_range[0],
                c.span_range[1],
                toml_str(&c.text)
            );
        }
        out.push_str("]\n");
        out.push_str("response_sentences = [\n");
        for r in &example.response_sentences {
            let cites = r
                .cited_spans
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "    {{ text = {}, cited_spans = [{}] }},",
                toml_str(&r.text),
                cites
            );
        }
        out.push_str("]\n");
        for m in &example.support_mutations {
            let _ = writeln!(out, "[[example.support_mutations]]");
            let _ = writeln!(out, "text = {}", toml_str(&m.text));
            let _ = writeln!(out, "class = {}", toml_str(class_label(m.class)));
        }
        out.push('\n');
    }
    Ok(out)
}

fn toml_str(s: &str) -> String {
    // TOML basic-string with quote/backslash escapes. We never embed
    // control characters; rejecting newlines keeps this simple.
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

fn class_label(class: evidence_eval::HallucinationClass) -> &'static str {
    use evidence_eval::HallucinationClass as C;
    match class {
        C::Valid => "valid",
        C::Uncited => "uncited",
        C::FabricatedSpan => "fabricated_span",
        C::OutOfContext => "out_of_context",
        C::Unsupported => "unsupported",
        C::Contradicted => "contradicted",
    }
}
