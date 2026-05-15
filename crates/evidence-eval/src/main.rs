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
    fetch_dailymed, inject_all_variants, load_dataset, run_with_mode, write_markdown, FetchConfig,
    InjectionConfig, Report, SupportMode, UreqClient,
};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "evidence-eval", about = "Closed-Loop Citation eval harness")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the validator harness on a labeled dataset.
    Run {
        /// Path to a TOML eval dataset.
        dataset: PathBuf,
        /// Optional output path for the markdown report. Default: stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Use the real NLI cross-encoder for the support gate. First
        /// run downloads `Xenova/distilbert-base-uncased-mnli` (~265 MB).
        #[arg(long)]
        real_nli: bool,
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
        } => run_command(&dataset, output.as_deref(), real_nli),
        Command::Inject { input, output } => inject_command(&input, &output),
        Command::Fetch { source } => match source {
            FetchSource::Dailymed {
                output,
                limit,
                delay_ms,
            } => fetch_dailymed_command(&output, limit, delay_ms),
        },
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
) -> Result<ExitCode> {
    let dataset = load_dataset(dataset_path).context("loading dataset")?;
    let mode = if real_nli {
        SupportMode::RealNli
    } else {
        SupportMode::Mock
    };
    let rows = run_with_mode(&dataset, mode).context("running eval")?;
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
