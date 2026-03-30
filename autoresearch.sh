#!/usr/bin/env bash
set -euo pipefail

TARGET_REPO="$HOME/Dev/wt-gph-rspec-rip-out"
NITROCOP_BIN="target/release/nitrocop"
RUBOCOP_BASELINE=3153

# Fast pre-checks
[[ -x "$NITROCOP_BIN" ]] || { echo "error: missing executable $NITROCOP_BIN" >&2; exit 2; }
[[ -d "$TARGET_REPO" ]] || { echo "error: missing target repo $TARGET_REPO" >&2; exit 2; }

start_ns=$(date +%s%N)
set +e
output="$($NITROCOP_BIN "$TARGET_REPO" 2>&1)"
status=$?
set -e
end_ns=$(date +%s%N)

runtime_ms=$(( (end_ns - start_ns) / 1000000 ))

# Preserve command output for debugging/context.
printf '%s\n' "$output"

# Treat normal lint exit code (1 = offenses found) as successful benchmark execution.
if [[ "$status" -ne 0 && "$status" -ne 1 ]]; then
  echo "error: nitrocop exited with status $status" >&2
  exit "$status"
fi

summary_line=$(printf '%s\n' "$output" | grep -E '[0-9]+ files inspected, [0-9]+ offenses detected' | tail -n1 || true)
if [[ -z "$summary_line" ]]; then
  echo "error: could not parse nitrocop offense summary" >&2
  exit 2
fi

nitrocop_violations=$(printf '%s\n' "$summary_line" | sed -E 's/.* inspected, ([0-9]+) offenses detected.*/\1/')
violation_gap=$(( nitrocop_violations > RUBOCOP_BASELINE ? nitrocop_violations - RUBOCOP_BASELINE : RUBOCOP_BASELINE - nitrocop_violations ))
runtime_budget_hit=$(( runtime_ms <= 150 ? 1 : 0 ))

# Structured metrics for run_experiment parser
printf 'METRIC violation_gap=%s\n' "$violation_gap"
printf 'METRIC nitrocop_violations=%s\n' "$nitrocop_violations"
printf 'METRIC nitrocop_ms=%s\n' "$runtime_ms"
printf 'METRIC runtime_budget_hit=%s\n' "$runtime_budget_hit"
