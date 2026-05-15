//! `evidence-eval` — run a TOML eval dataset through every
//! [`ValidationPolicy`] and print a markdown report.
//!
//! ```sh
//! cargo run -p evidence-eval -- crates/evidence-eval/datasets/bootstrap.toml
//! cargo run -p evidence-eval -- crates/evidence-eval/datasets/bootstrap.toml --output report.md
//! ```

#![deny(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use evidence_eval::{load_dataset, run_with_mode, write_markdown, Report, SupportMode};

#[derive(Parser)]
#[command(name = "evidence-eval", about = "Closed-Loop Citation eval harness")]
struct Cli {
    /// Path to a TOML eval dataset.
    dataset: PathBuf,
    /// Optional output path for the markdown report. Default: stdout.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Use the real NLI cross-encoder for the support gate. First run
    /// downloads `Xenova/distilbert-base-uncased-mnli` (~265 MB).
    /// Default is the class-derived mocks, which are deterministic and
    /// fast but only measure validator wiring.
    #[arg(long)]
    real_nli: bool,
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
    let dataset = load_dataset(&cli.dataset).context("loading dataset")?;
    let mode = if cli.real_nli {
        SupportMode::RealNli
    } else {
        SupportMode::Mock
    };
    let rows = run_with_mode(&dataset, mode).context("running eval")?;
    let report = Report::new(rows);
    let md = write_markdown(&report);

    if let Some(out_path) = cli.output {
        std::fs::write(&out_path, md.as_bytes()).context("writing report")?;
        eprintln!("wrote report to {}", out_path.display());
    } else {
        print!("{md}");
    }

    // Exit code 2 if any row disagrees with the expected matrix — useful
    // for CI / regression catches.
    let (agree, total) = report.agreement();
    if agree == total {
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!(
            "disagreement: {} / {} rows match expected matrix",
            agree, total
        );
        Ok(ExitCode::from(2))
    }
}
