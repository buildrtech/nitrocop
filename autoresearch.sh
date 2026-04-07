#!/usr/bin/env bash
set -euo pipefail

TARGET_PATH="${1:-$HOME/Dev/wt-gph-rspec-rip-out}"
EXPECTED_OFFENSES=3992
BIN="target/release/nitrocop"
BUILD_STAMP="target/release/.nitrocop_autoresearch_stamp"
RELEVANT_PATHS=(src bench Cargo.toml Cargo.lock)

current_build_fingerprint() {
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "nogit"
    return
  fi

  local dirty=0
  if ! git diff --quiet -- "${RELEVANT_PATHS[@]}"; then
    dirty=1
  fi
  if ! git diff --cached --quiet -- "${RELEVANT_PATHS[@]}"; then
    dirty=1
  fi
  if git ls-files --others --exclude-standard -- "${RELEVANT_PATHS[@]}" | grep -q .; then
    dirty=1
  fi

  if [[ "$dirty" -eq 0 ]]; then
    echo "head:$(git rev-parse HEAD)"
    return
  fi

  {
    git diff -- "${RELEVANT_PATHS[@]}"
    git diff --cached -- "${RELEVANT_PATHS[@]}"
    git ls-files --others --exclude-standard -- "${RELEVANT_PATHS[@]}"
  } | shasum -a 256 | awk '{print "dirty:" $1}'
}

rebuild_if_needed() {
  local fingerprint
  fingerprint="$(current_build_fingerprint)"

  if [[ -x "$BIN" ]] && [[ -f "$BUILD_STAMP" ]] && [[ "$(<"$BUILD_STAMP")" == "$fingerprint" ]]; then
    return
  fi

  cargo build --release >/dev/null
  mkdir -p "$(dirname "$BUILD_STAMP")"
  printf '%s\n' "$fingerprint" >"$BUILD_STAMP"
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
