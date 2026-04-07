#!/usr/bin/env bash
set -euo pipefail

TARGET_PATH="${1:-$HOME/Dev/wt-gph-rspec-rip-out}"
EXPECTED_OFFENSES=3992
BIN="target/release/nitrocop"

rebuild_if_needed() {
  if [[ ! -x "$BIN" ]]; then
    cargo build --release >/dev/null
    return
  fi

  if find src bench Cargo.toml Cargo.lock -type f -newer "$BIN" -print -quit | grep -q .; then
    cargo build --release >/dev/null
  fi
}

run_once_ms() {
  local out err rc offense real_s ms
  out="$(mktemp)"
  err="$(mktemp)"

  set +e
  /usr/bin/time -p "$BIN" --no-cache "$TARGET_PATH" >"$out" 2>"$err"
  rc=$?
  set -e

  # nitrocop returns 1 when offenses are found.
  if [[ "$rc" -ne 0 && "$rc" -ne 1 ]]; then
    echo "nitrocop failed with unexpected exit code: $rc" >&2
    cat "$err" >&2
    rm -f "$out" "$err"
    return 2
  fi

  offense="$(uv run python - <<'PY' "$out"
import re, sys
text = open(sys.argv[1], encoding='utf-8', errors='replace').read()
m = re.search(r'(\d+)\s+offenses?\s+detected', text)
print(m.group(1) if m else '')
PY
)"

  if [[ -z "$offense" ]]; then
    echo "Could not parse offense count from output" >&2
    tail -n 20 "$out" >&2 || true
    rm -f "$out" "$err"
    return 3
  fi

  if [[ "$offense" -ne "$EXPECTED_OFFENSES" ]]; then
    echo "Offense invariant violated: expected $EXPECTED_OFFENSES, got $offense" >&2
    rm -f "$out" "$err"
    return 4
  fi

  real_s="$(awk '/^real /{print $2}' "$err")"
  if [[ -z "$real_s" ]]; then
    echo "Could not parse wall time from /usr/bin/time output" >&2
    cat "$err" >&2
    rm -f "$out" "$err"
    return 5
  fi

  ms="$(uv run python - <<'PY' "$real_s"
import sys
print(f"{float(sys.argv[1])*1000:.2f}")
PY
)"

  echo "$ms"
  rm -f "$out" "$err"
}

median_three() {
  local a b c
  a="$(run_once_ms)"
  b="$(run_once_ms)"
  c="$(run_once_ms)"
  printf '%s\n%s\n%s\n' "$a" "$b" "$c" | sort -n | sed -n '2p'
}

rebuild_if_needed
median="$(median_three)"
echo "METRIC total_ms=$median"
