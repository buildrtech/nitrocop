#!/usr/bin/env bash
set -euo pipefail

TARGET_DIR="${TARGET_DIR:-$HOME/Dev/wt-gph-rspec-rip-out}"
EXPECTED_OFFENSES=3992
BIN="./target/release/nitrocop"
OUT_JSON="/tmp/nitrocop-autoresearch-output.json"

if [[ ! -x "$BIN" ]]; then
  echo "nitrocop binary missing at $BIN; build with: cargo build --release" >&2
  exit 2
fi

start_s=$(perl -MTime::HiRes=time -e 'print time')
set +e
"$BIN" --format json "$TARGET_DIR" > "$OUT_JSON"
cmd_status=$?
set -e
end_s=$(perl -MTime::HiRes=time -e 'print time')

# nitrocop returns non-zero when offenses are found; tolerate that.
if [[ $cmd_status -ne 0 && $cmd_status -ne 1 ]]; then
  echo "nitrocop failed with exit code $cmd_status" >&2
  exit $cmd_status
fi

offense_count=$(jq '.offenses | length' "$OUT_JSON")
json_bytes=$(wc -c < "$OUT_JSON" | tr -d ' ')

total_ms=$(awk -v s="$start_s" -v e="$end_s" 'BEGIN { printf "%.3f", (e-s)*1000 }')

echo "METRIC total_ms=$total_ms"
echo "METRIC offense_count=$offense_count"
echo "METRIC json_bytes=$json_bytes"

if [[ "$offense_count" -ne "$EXPECTED_OFFENSES" ]]; then
  echo "Offense count changed: expected $EXPECTED_OFFENSES, got $offense_count" >&2
  exit 3
fi
