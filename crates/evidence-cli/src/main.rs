//! `evidence` — headless CLI for the evidence research assistant.
//!
//! Two commands:
//! - `evidence ingest <pdf>` parses the PDF, writes pages/spans/chunks, and
//!   embeds chunks into the local SQLite index.
//! - `evidence query <text>` runs the hybrid retriever, asks the configured
//!   LLM for an answer with span-level citations, and prints the result.
//!   Exits with code 2 if the validator refuses the answer.

#![deny(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use evidence_cli::commands::{ingest, query as query_cmd};
use evidence_cli::ollama::OllamaBackend;
use evidence_core::query::{
    LlmBackend, NliCrossEncoder, NliSupportChecker, RerankerSupportChecker, SupportChecker,
};
use evidence_core::retrieval::{BgeReranker, BgeSmall};
use evidence_core::storage::Storage;

/// Which backend to use for the citation lexical-support check when
/// `--check-support` is set.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SupportMode {
    /// Cross-encoder reranker as a relevance proxy (v0.2 default). Only
    /// emits `Supports` or `Neutral` — can't tell contradiction apart.
    Rerank,
    /// Real NLI cross-encoder (v0.3). Emits all three verdicts.
    Nli,
}

/// Holds whichever backend `--support-mode` selected, keeping it alive
/// for the duration of the borrow into the support checker.
enum SupportBackend {
    Rerank(BgeReranker),
    Nli(NliCrossEncoder),
}

#[derive(Parser)]
#[command(
    name = "evidence",
    version,
    about = "Auditable AI research assistant — span-level citations or refusal."
)]
struct Cli {
    /// Path to the SQLite index. Defaults to ./evidence.db.
    #[arg(long, global = true, default_value = "evidence.db")]
    db: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Ingest a PDF: parse pages and spans, chunk, embed, and write to the index.
    Ingest {
        /// PDF path on disk.
        path: PathBuf,
        /// Optional human-readable title to store with the document.
        #[arg(long)]
        title: Option<String>,
    },
    /// Ask a question of the indexed corpus.
    Query {
        /// The question.
        question: String,
        /// How many chunks to retrieve and show the model.
        #[arg(long, default_value_t = 8)]
        k: usize,
        /// Ollama base URL. Defaults to the upstream local default.
        #[arg(long, default_value_t = evidence_cli::ollama::DEFAULT_BASE_URL.to_string())]
        ollama_url: String,
        /// Ollama model name.
        #[arg(long, default_value_t = evidence_cli::ollama::DEFAULT_MODEL.to_string())]
        model: String,
        /// Enable the citation lexical-support check. First use downloads
        /// the support-mode model (~265 MB for `nli`, ~280 MB for
        /// `rerank`).
        #[arg(long)]
        check_support: bool,
        /// Which checker to use under `--check-support`. The NLI checker
        /// emits all three verdicts (supports / neutral / contradicts);
        /// the reranker checker is the v0.2 proxy that only emits two.
        #[arg(long, value_enum, default_value_t = SupportMode::Nli)]
        support_mode: SupportMode,
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
        Command::Ingest { path, title } => {
            let mut storage = Storage::open(&cli.db).context("opening evidence index")?;
            let embedder = BgeSmall::new().context("initializing bge-small embedder")?;
            let summary = ingest::run(&mut storage, &embedder, &path, title.as_deref())
                .context("ingesting PDF")?;
            println!(
                "ingested doc#{} sha256={} pages={} spans={} chunks={}",
                summary.document_id,
                &summary.sha256[..16],
                summary.page_count,
                summary.span_count,
                summary.chunk_count,
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Query {
            question,
            k,
            ollama_url,
            model,
            check_support,
            support_mode,
        } => {
            let storage = Storage::open(&cli.db).context("opening evidence index")?;
            let embedder = BgeSmall::new().context("initializing bge-small embedder")?;
            let llm: Box<dyn LlmBackend> = Box::new(OllamaBackend::new(ollama_url, model));

            // The support checker borrows the backend it wraps, so both
            // have to live for the duration of the call.
            let backend = if check_support {
                Some(match support_mode {
                    SupportMode::Rerank => SupportBackend::Rerank(
                        BgeReranker::new().context("initializing bge-reranker-base")?,
                    ),
                    SupportMode::Nli => SupportBackend::Nli(
                        NliCrossEncoder::new().context("initializing NLI cross-encoder")?,
                    ),
                })
            } else {
                None
            };
            let checker: Option<Box<dyn SupportChecker>> = backend.as_ref().map(|b| match b {
                SupportBackend::Rerank(r) => {
                    Box::new(RerankerSupportChecker::new(r)) as Box<dyn SupportChecker>
                }
                SupportBackend::Nli(n) => {
                    Box::new(NliSupportChecker::new(n)) as Box<dyn SupportChecker>
                }
            });

            match query_cmd::run(
                &storage,
                &embedder,
                llm.as_ref(),
                checker.as_deref(),
                &question,
                k,
            ) {
                Ok(answer) => {
                    print!("{}", query_cmd::format_answer(&answer));
                    Ok(ExitCode::SUCCESS)
                }
                Err(err) => {
                    eprint!("{}", query_cmd::format_refusal(&err));
                    Ok(ExitCode::from(2))
                }
            }
        }
    }
}
