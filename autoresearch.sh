#!/usr/bin/env bash
set -euo pipefail

TARGET_DIR="${TARGET_DIR:-$HOME/Dev/wt-gph-rspec-rip-out}"
EXPECTED_OFFENSES=3992
BIN="./target/release/nitrocop"
OUT_TXT="/tmp/nitrocop-autoresearch-output.txt"

# Ensure benchmark uses the latest source changes. Build cost is excluded from metric.
cargo build --release -q

# Ensure benchmark uses the latest source changes without perturbing run-time
# caches when no rebuild is needed.
needs_build=0
if [[ ! -x "$BIN" ]]; then
  needs_build=1
elif find src Cargo.toml Cargo.lock -type f -newer "$BIN" | head -n 1 | grep -q .; then
  needs_build=1
fi

if [[ $needs_build -eq 1 ]]; then
  cargo build --release -q
fi

if [[ ! -x "$BIN" ]]; then
  echo "nitrocop binary missing at $BIN; build with: cargo build --release" >&2
  exit 2
fi

start_s=$(perl -MTime::HiRes=time -e 'print time')
set +e
"$BIN" "$TARGET_DIR" > "$OUT_TXT" 2>&1
cmd_status=$?
set -e
end_s=$(perl -MTime::HiRes=time -e 'print time')

# nitrocop returns non-zero when offenses are found; tolerate that.
if [[ $cmd_status -ne 0 && $cmd_status -ne 1 ]]; then
  echo "nitrocop failed with exit code $cmd_status" >&2
  tail -n 20 "$OUT_TXT" >&2 || true
  exit $cmd_status
fi

summary_line=$(rg -N "files inspected, .* offenses detected" "$OUT_TXT" | tail -n 1)
if [[ -z "$summary_line" ]]; then
  echo "Failed to parse summary line from nitrocop output" >&2
  tail -n 20 "$OUT_TXT" >&2 || true
  exit 4
fi

offense_count=$(echo "$summary_line" | sed -E 's/.* ([0-9][0-9,]*) offenses detected.*/\1/' | tr -d ',')
output_bytes=$(wc -c < "$OUT_TXT" | tr -d ' ')

total_ms=$(awk -v s="$start_s" -v e="$end_s" 'BEGIN { printf "%.3f", (e-s)*1000 }')

echo "METRIC total_ms=$total_ms"
echo "METRIC offense_count=$offense_count"
echo "METRIC output_bytes=$output_bytes"

if [[ "$offense_count" -ne "$EXPECTED_OFFENSES" ]]; then
  echo "Offense count changed: expected $EXPECTED_OFFENSES, got $offense_count" >&2
  exit 3
fi
