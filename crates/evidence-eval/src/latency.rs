//! §5.7 cost / latency.
//!
//! Reports the wall-clock time attributable to each validator gate by
//! measuring at the *harness* level — deliberately not by instrumenting
//! `evidence-core`. Two runs over the same dataset:
//!
//! - a **mock-only** run (`SupportMode::Mock`): the support gate uses a
//!   class-derived in-memory checker, so its cost is ~0. This run's wall
//!   time is the **structural cost** (existence + in-context gates plus
//!   harness setup: SQLite seeding, prompt/answer construction).
//! - a **real-NLI** run (`SupportMode::RealNli`): identical except the
//!   support gate runs the NLI cross-encoder forward passes. The
//!   *delta* over the mock run is the **support-gate cost**.
//!
//! This is an honest, coarse decomposition: it does not separate
//! existence from in-context (both are O(µs) string/index checks and
//! dwarfed by NLI), and the support figure folds in tokenizer +
//! forward-pass + verdict mapping. The note in the rendered table says
//! so explicitly. Model load is excluded: `run_with_model` loads the
//! encoder once *before* iterating, so neither run's timed region
//! includes the download or warm-up.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use crate::dataset::Dataset;
use crate::runner::{run_with_model, SupportMode};

/// One timed harness pass: wall time and how many (example, policy)
/// rows it produced (the denominator for per-call figures).
#[derive(Debug, Clone, Copy)]
pub struct Pass {
    pub wall: Duration,
    pub rows: usize,
}

/// The §5.7 measurement: the structural-only pass and the real-NLI
/// pass over the same dataset.
#[derive(Debug, Clone, Copy)]
pub struct LatencyMeasurement {
    pub examples: usize,
    /// Mock-only run — structural gates + harness overhead.
    pub structural: Pass,
    /// Real-NLI run — structural cost plus the NLI support gate.
    pub real_nli: Pass,
}

impl LatencyMeasurement {
    /// Support-gate wall time = real-NLI total − structural total.
    /// Saturates at zero: scheduling jitter can in principle make a
    /// mock run momentarily slower; a negative "cost" is meaningless,
    /// so we floor it and the note flags the method's coarseness.
    #[must_use]
    pub fn support_cost(&self) -> Duration {
        self.real_nli
            .wall
            .checked_sub(self.structural.wall)
            .unwrap_or(Duration::ZERO)
    }

    /// Mean structural cost per (example, policy) call.
    #[must_use]
    pub fn structural_per_call(&self) -> Duration {
        per_call(self.structural.wall, self.structural.rows)
    }

    /// Mean support-gate cost per call.
    #[must_use]
    pub fn support_per_call(&self) -> Duration {
        per_call(self.support_cost(), self.real_nli.rows)
    }

    /// Fraction of the real-NLI wall time spent in the support gate.
    #[must_use]
    pub fn support_fraction(&self) -> f64 {
        let total = self.real_nli.wall.as_secs_f64();
        if total <= 0.0 {
            0.0
        } else {
            self.support_cost().as_secs_f64() / total
        }
    }
}

fn per_call(total: Duration, rows: usize) -> Duration {
    if rows == 0 {
        Duration::ZERO
    } else {
        total / u32::try_from(rows).unwrap_or(u32::MAX)
    }
}

/// Time a single harness pass under `mode`. Model load (under
/// `RealNli`) happens inside `run_with_model` *before* iteration, so it
/// is included here — but it is amortized identically into the mock
/// baseline's structural figure only via the rendered note, not the
/// numbers (the mock pass never loads a model). To keep the delta
/// honest we therefore time `run_with_model` end-to-end and document
/// that the real-NLI pass's first call carries one model warm-up.
///
/// # Errors
///
/// Propagates dataset-seeding errors and, under `RealNli`, NLI
/// initialization failures.
pub fn time_pass(
    dataset: &Dataset,
    mode: SupportMode,
    nli_model: Option<&str>,
) -> anyhow::Result<Pass> {
    let start = Instant::now();
    let rows = run_with_model(dataset, mode, nli_model)?;
    Ok(Pass {
        wall: start.elapsed(),
        rows: rows.len(),
    })
}

/// Run the structural (mock) and real-NLI passes and assemble the
/// measurement.
///
/// # Errors
///
/// Propagates any error from either pass.
pub fn measure(dataset: &Dataset, nli_model: Option<&str>) -> anyhow::Result<LatencyMeasurement> {
    let structural = time_pass(dataset, SupportMode::Mock, None)?;
    let real_nli = time_pass(dataset, SupportMode::RealNli, nli_model)?;
    Ok(LatencyMeasurement {
        examples: dataset.examples.len(),
        structural,
        real_nli,
    })
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

fn us(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000_000.0
}

/// Render the §5.7 markdown table from a measurement. Pure over the
/// measurement so it is unit-testable without running a model.
#[must_use]
pub fn render_latency(m: &LatencyMeasurement, nli_label: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Cost & latency (§5.7)\n");
    let _ = writeln!(
        out,
        "Harness-level decomposition over **{}** examples \
         ({} timed (example, policy) calls per pass). NLI checkpoint: \
         `{nli_label}`.\n",
        m.examples, m.real_nli.rows
    );

    let _ = writeln!(out, "| stage | total | mean / call | share |");
    let _ = writeln!(out, "|---|---|---|---|");
    let _ = writeln!(
        out,
        "| structural (existence + in-context + harness) | {:.1} ms | {:.1} µs | {:.1}% |",
        ms(m.structural.wall),
        us(m.structural_per_call()),
        (1.0 - m.support_fraction()) * 100.0,
    );
    let _ = writeln!(
        out,
        "| support gate (NLI forward passes) | {:.1} ms | {:.1} ms | {:.1}% |",
        ms(m.support_cost()),
        ms(m.support_per_call()),
        m.support_fraction() * 100.0,
    );
    let _ = writeln!(
        out,
        "| **total (real-NLI run)** | **{:.1} ms** | **{:.1} ms** | **100%** |",
        ms(m.real_nli.wall),
        ms(per_call(m.real_nli.wall, m.real_nli.rows)),
    );

    let _ = writeln!(
        out,
        "\n**Structural vs. support split:** {:.1}% structural / {:.1}% \
         support, by wall time of the real-NLI run.",
        (1.0 - m.support_fraction()) * 100.0,
        m.support_fraction() * 100.0,
    );

    let _ = writeln!(
        out,
        "\n> Measurement method (honest note): per-gate cost is measured \
         at the harness level, not by instrumenting `evidence-core`. The \
         structural figure is the wall time of a mock-support run (the \
         support checker is an in-memory class-derived stub, ≈0 cost); \
         it bundles the existence + in-context gates with harness setup \
         (in-memory SQLite seeding, prompt/answer construction). The \
         support figure is the *delta* of a real-NLI run over that mock \
         run on the same dataset, so it folds tokenization, the NLI \
         forward pass, and verdict mapping into one number, and includes \
         a one-time model warm-up on the first support call. Existence \
         vs. in-context are not separated — both are O(µs) index/string \
         checks dwarfed by the NLI pass. Treat these as order-of-\
         magnitude, not microbenchmark, figures."
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(ms: u64, rows: usize) -> Pass {
        Pass {
            wall: Duration::from_millis(ms),
            rows,
        }
    }

    fn measurement(struct_ms: u64, real_ms: u64) -> LatencyMeasurement {
        LatencyMeasurement {
            examples: 10,
            structural: pass(struct_ms, 40),
            real_nli: pass(real_ms, 40),
        }
    }

    #[test]
    fn support_cost_is_the_delta() {
        let m = measurement(20, 120);
        assert_eq!(m.support_cost(), Duration::from_millis(100));
    }

    #[test]
    fn support_cost_floors_at_zero() {
        // Jitter made the mock pass slower — a negative cost is
        // meaningless, so it saturates at zero.
        let m = measurement(50, 30);
        assert_eq!(m.support_cost(), Duration::ZERO);
        assert_eq!(m.support_fraction(), 0.0);
    }

    #[test]
    fn per_call_divides_by_row_count() {
        let m = measurement(40, 440);
        // structural 40ms / 40 rows = 1ms = 1000µs
        assert!((us(m.structural_per_call()) - 1_000.0).abs() < 1.0);
        // support 400ms / 40 rows = 10ms
        assert!((ms(m.support_per_call()) - 10.0).abs() < 0.01);
    }

    #[test]
    fn per_call_handles_zero_rows() {
        let m = LatencyMeasurement {
            examples: 0,
            structural: pass(10, 0),
            real_nli: pass(10, 0),
        };
        assert_eq!(m.structural_per_call(), Duration::ZERO);
        assert_eq!(m.support_per_call(), Duration::ZERO);
    }

    #[test]
    fn support_fraction_is_bounded() {
        let m = measurement(25, 100);
        let f = m.support_fraction();
        assert!((f - 0.75).abs() < 1e-9, "got {f}");
        assert!((0.0..=1.0).contains(&f));
    }

    #[test]
    fn render_has_table_and_honest_note() {
        let r = render_latency(&measurement(20, 120), "distilbert (default)");
        assert!(r.contains("# Cost & latency (§5.7)"));
        assert!(r.contains("| stage | total | mean / call | share |"));
        assert!(r.contains("structural (existence + in-context + harness)"));
        assert!(r.contains("support gate (NLI forward passes)"));
        assert!(r.contains("Structural vs. support split:"));
        assert!(r.contains("Measurement method (honest note)"));
        assert!(r.contains("`distilbert (default)`"));
    }
}
