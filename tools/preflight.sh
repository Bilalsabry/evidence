#!/usr/bin/env bash
# tools/preflight.sh
#
# Pre-submission preflight: mechanically verifies every Phase 6 check
# that can be machine-checked, in order, and exits non-zero with a clear
# message at the first failure.
#
# Usage:
#   bash tools/preflight.sh
#
# Idempotent and safe to re-run. Run from the repo root.
# Requires only what the repo already needs: cargo, git, grep, diff.
# latexmk is optional (paper/ build is skipped with a warning if absent).

set -u
set -o pipefail

# ---------- helpers ----------
RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; BOLD=$'\033[1m'; RESET=$'\033[0m'
PASSED=()

step() { printf '\n%s==> %s%s\n' "$BOLD" "$*" "$RESET"; }
ok()   { printf '%s  PASS%s %s\n' "$GREEN" "$RESET" "$*"; PASSED+=("$*"); }
fail() {
  printf '\n%sFAIL%s %s\n' "$RED" "$RESET" "$*" >&2
  printf '\nPassed before this:\n' >&2
  for p in "${PASSED[@]:-}"; do printf '  - %s\n' "$p" >&2; done
  exit 1
}
warn() { printf '%sWARN%s %s\n' "$YELLOW" "$RESET" "$*"; }

# Resolve repo root so the script can be invoked from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT" || fail "could not cd to repo root: $REPO_ROOT"

TMPDIR_PREFLIGHT="$(mktemp -d -t preflight-XXXXXX)"
trap 'rm -rf "$TMPDIR_PREFLIGHT"' EXIT

DATASET="crates/evidence-eval/datasets/fda_label_bench.toml"
CORPUS="${EVIDENCE_CORPUS:-$HOME/dailymed-corpus}"
NLI_MODEL="lquint/DeBERTa-v3-base-mnli-fever-anli-onnx"

# ---------- 1. clean main ----------
step "1/10  Repo clean and on main"
BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '')"
[ "$BRANCH" = "main" ] || fail "not on main (HEAD=$BRANCH). Switch with: git checkout main"
PORC="$(git status --porcelain)"
[ -z "$PORC" ] || fail $'working tree not clean:\n'"$PORC"
ok "git clean on main"

# ---------- 2. fmt / clippy / test ----------
step "2/10  cargo fmt --check / clippy / test"
cargo fmt --check                                                                            || fail "cargo fmt --check failed"
cargo clippy --workspace --exclude evidence-desktop --all-targets -- -D warnings             || fail "cargo clippy failed (warnings denied)"
cargo test  -p evidence-eval                                                                  || fail "cargo test -p evidence-eval failed"
ok "fmt + clippy + tests"

# ---------- 3. dataset lint ----------
step "3/10  Dataset lint"
cargo run --quiet -p evidence-eval -- lint "$DATASET" || fail "evidence-eval lint failed for $DATASET"
ok "dataset lint clean"

# ---------- 4. faithfulness audit re-run + compare to committed header ----------
step "4/10  Faithfulness audit re-run vs committed header"
[ -d "$CORPUS" ] || fail "corpus not found at $CORPUS (override with EVIDENCE_CORPUS=<path>)"
AUDIT_OUT="$TMPDIR_PREFLIGHT/faithfulness-audit.md"
# Exit 2 here means the tool flagged "missing" spans. The committed
# audit (docs/paper/faithfulness-audit.md) documents the baseline +
# manual per-span verdicts. We accept exit 0 or 2 here; the count
# comparison below is the real check (must match the committed
# header exactly — if a NEW miss appears, that fails the comparison
# and is the real signal we care about).
cargo run --quiet -p evidence-eval -- audit-faithfulness \
  --dataset "$DATASET" --corpus "$CORPUS" --output "$AUDIT_OUT" \
  || true
[ -s "$AUDIT_OUT" ] || fail "audit-faithfulness produced no output"

extract_total() {
  # extract the integer for a "Totals" bullet like "- missing: 5" from a file
  local file="$1" key="$2"
  grep -E "^- ${key}:" "$file" | head -1 | grep -oE '[0-9]+' | head -1
}
COMMITTED_AUDIT="docs/paper/faithfulness-audit.md"
[ -f "$COMMITTED_AUDIT" ] || fail "committed audit not found: $COMMITTED_AUDIT"
for key in examples 'corpus spans' exact fuzzy missing; do
  a="$(extract_total "$AUDIT_OUT" "$key")"
  b="$(extract_total "$COMMITTED_AUDIT" "$key")"
  [ -n "$a" ] && [ -n "$b" ] || fail "could not parse '$key' total in audit (regen='$a' committed='$b')"
  [ "$a" = "$b" ] || fail "faithfulness '$key' mismatch: regen=$a committed=$b"
done
ok "faithfulness totals match committed audit"

# ---------- 5. no fabrication-flag tokens ----------
step "5/10  No fabrication-flag tokens in docs/ or paper/"
FLAG_RE='\[verify\]|\[unverified|TODO|FIXME|FILL_IN|Lorem'
# Exclude:
#   - third-party ACL template files (acl.sty, acl_natbib.bst)
#   - self-referential mentions in SUBMISSION_RUNBOOK / preflight docs
#     (those documents intentionally NAME the flag tokens as things
#     reviewers/scripts look for; matching their own text is a false
#     positive)
#   - schema-placeholder names like FILL_IN_HUMAN_LABEL described in
#     the natfail harness/protocol docs (the docs ABOUT the placeholder
#     are not themselves the placeholder)
EXCLUDE_FLAG='paper/acl\.sty|paper/acl_natbib\.bst|docs/paper/SUBMISSION_RUNBOOK\.md|docs/paper/natural-failure-harness\.md|docs/paper/natural-failure-protocol\.md'
HITS="$(grep -rnE "$FLAG_RE" docs/ paper/ 2>/dev/null | grep -vE "$EXCLUDE_FLAG" || true)"
if [ -n "$HITS" ]; then
  printf '%s\n' "$HITS" >&2
  fail "fabrication-flag tokens found (see lines above)"
fi
ok "no fabrication-flag tokens"

# ---------- 6. no overclaim language ----------
step "6/10  No overclaim language"
OVER_RE='\bproven\b|perfectly disjoint|strongest possible|machine-drafted|machine-assembled'
# Allowlist: files that legitimately discuss these terms.
ALLOW_OVER='docs/paper/PROVENANCE\.md|docs/paper/.*_PROVENANCE\.md|docs/paper/faithfulness-audit\.md|docs/paper/citation-audit\.md'
HITS="$(grep -rniE "$OVER_RE" docs/ README.md 2>/dev/null | grep -vE "$ALLOW_OVER" || true)"
if [ -n "$HITS" ]; then
  printf '%s\n' "$HITS" >&2
  fail "overclaim language found (see lines above)"
fi
ok "no overclaim language"

# ---------- 7. no unexplained AI-tool attribution ----------
step "7/10  No AI-tool attribution"
ATTR_RE='co-authored|claude'
# Allow: legit `Claude Haiku` eval-model line, and PROVENANCE / DATASHEET
# disclosure paragraphs which legitimately discuss authoring tools.
ALLOW_ATTR='Claude Haiku|docs/paper/.*PROVENANCE\.md|docs/paper/DATASHEET\.md|docs/paper/INTEGRITY\.md'
HITS="$(grep -rniE "$ATTR_RE" docs/ README.md crates/evidence-eval/src 2>/dev/null \
        | grep -viE "$ALLOW_ATTR" || true)"
if [ -n "$HITS" ]; then
  printf '%s\n' "$HITS" >&2
  fail "unexplained AI-tool attribution found (see lines above)"
fi
ok "no unexplained AI-tool attribution"

# ---------- 8. number trace: regen metrics/compare/latency + diff ----------
step "8/10  Number trace: regen metrics/compare/latency and diff"
INJ="$TMPDIR_PREFLIGHT/inj.toml"
cargo run --quiet -p evidence-eval -- inject "$DATASET" --output "$INJ" \
  || fail "inject failed"

METRICS_OUT="$TMPDIR_PREFLIGHT/rule-metrics.md"
COMPARE_OUT="$TMPDIR_PREFLIGHT/nli-comparison.md"
LATENCY_OUT="$TMPDIR_PREFLIGHT/cost-latency.md"

cargo run --quiet -p evidence-eval -- metrics "$INJ" --real-nli \
  --nli-model "$NLI_MODEL" --output "$METRICS_OUT" \
  || fail "metrics regen failed"
cargo run --quiet -p evidence-eval -- compare "$INJ" \
  --candidate-nli "$NLI_MODEL" --output "$COMPARE_OUT" \
  || fail "compare regen failed"
cargo run --quiet -p evidence-eval -- latency --dataset "$INJ" --real-nli \
  --nli-model "$NLI_MODEL" --output "$LATENCY_OUT" \
  || fail "latency regen failed"

# Diff committed numbers vs regen. Numbers = any sequence of digits
# (optionally with a decimal point or % sign). We compare the SET of
# numeric tokens per file — line-order changes in tables are tolerated,
# but any committed number that disappears or any new number that
# appears is a failure. Latency wall-clock totals are inherently
# noisy, so for cost-latency.md we relax to "% shares and example
# counts must match" by stripping ms/µs/s magnitudes.
extract_nums() {
  grep -oE '[0-9]+(\.[0-9]+)?%?' "$1" | sort
}
extract_nums_latency() {
  # Drop raw timings (ms / µs / s) AND percentage shares — both are
  # machine-dependent (a faster machine spends a larger share in NLI;
  # a slower one a smaller one). The load-bearing claim that survives
  # is dataset shape (1,270 examples, 5,080 rows). Strict-match those.
  sed -E 's/[0-9]+(\.[0-9]+)?[[:space:]]*(ms|µs|us|s)//g; s/[0-9]+(\.[0-9]+)?%//g' "$1" \
    | grep -oE '[0-9]+(\.[0-9]+)?' | sort
}

diff_nums() {
  local label="$1" committed="$2" regen="$3" mode="${4:-strict}"
  local ca ra
  ca="$TMPDIR_PREFLIGHT/${label}.committed.nums"
  ra="$TMPDIR_PREFLIGHT/${label}.regen.nums"
  if [ "$mode" = "latency" ]; then
    extract_nums_latency "$committed" > "$ca"
    extract_nums_latency "$regen"     > "$ra"
  else
    extract_nums "$committed" > "$ca"
    extract_nums "$regen"     > "$ra"
  fi
  if ! diff -u "$ca" "$ra" > "$TMPDIR_PREFLIGHT/${label}.diff"; then
    cat "$TMPDIR_PREFLIGHT/${label}.diff" >&2
    fail "$label: committed numbers differ from regenerated"
  fi
}

diff_nums "rule-metrics"   "docs/paper/rule-metrics.md"   "$METRICS_OUT" strict
diff_nums "nli-comparison" "docs/paper/nli-comparison.md" "$COMPARE_OUT" strict
diff_nums "cost-latency"   "docs/paper/cost-latency.md"   "$LATENCY_OUT" latency
ok "metrics/compare/latency numbers reproduce"

# ---------- 9. latexmk build (if paper/ has a .tex) ----------
step "9/10  paper/ LaTeX build (optional)"
TEX_MAIN=""
if [ -d paper ]; then
  TEX_MAIN="$(find paper -maxdepth 2 -name '*.tex' -type f | head -1 || true)"
fi
if [ -z "${TEX_MAIN:-}" ]; then
  warn "no .tex found under paper/; skipping latexmk"
elif ! command -v latexmk >/dev/null 2>&1; then
  warn "latexmk not installed; skipping PDF build"
else
  ( cd "$(dirname "$TEX_MAIN")" && latexmk -pdf -interaction=nonstopmode \
      "$(basename "$TEX_MAIN")" >/dev/null ) \
    || fail "latexmk -pdf failed for $TEX_MAIN"
  ok "latexmk -pdf succeeded ($TEX_MAIN)"
fi
[ -n "${TEX_MAIN:-}" ] && command -v latexmk >/dev/null 2>&1 || ok "paper build step handled (warned/skipped)"

# ---------- 10. summary ----------
step "10/10  Preflight summary"
printf '%s\n' "${GREEN}All preflight checks passed.${RESET}"
for p in "${PASSED[@]}"; do
  printf '  %s✓%s %s\n' "$GREEN" "$RESET" "$p"
done

exit 0
