#!/usr/bin/env bash
set -euo pipefail

TARGET_REPO="${NITROCOP_TARGET_REPO:-$HOME/Dev/wt-gph-rspec-rip-out}"
RUBOCOP_COUNT_FILE=".autoresearch_rubocop_count"
RUBOCOP_TARGET="${NITROCOP_RUBOCOP_TARGET:-}"
RUNS="${NITROCOP_BENCH_RUNS:-3}"
RUNTIME_BUDGET_MS="${NITROCOP_RUNTIME_BUDGET_MS:-150}"

if [[ -z "${RUBOCOP_TARGET}" ]]; then
  if [[ -f "${RUBOCOP_COUNT_FILE}" ]]; then
    RUBOCOP_TARGET="$(tr -d '[:space:]' < "${RUBOCOP_COUNT_FILE}")"
  else
    echo "error: missing ${RUBOCOP_COUNT_FILE}; run RuboCop once and store offense count" >&2
    exit 2
  fi
fi

if [[ ! -d "${TARGET_REPO}" ]]; then
  echo "error: target repo not found: ${TARGET_REPO}" >&2
  exit 2
fi

if ! [[ "${RUBOCOP_TARGET}" =~ ^[0-9]+$ ]]; then
  echo "error: invalid rubocop target count: ${RUBOCOP_TARGET}" >&2
  exit 2
fi

if ! [[ "${RUNS}" =~ ^[0-9]+$ ]] || [[ "${RUNS}" -lt 1 ]]; then
  echo "error: NITROCOP_BENCH_RUNS must be a positive integer" >&2
  exit 2
fi

# Keep binary current with code edits (incremental rebuild is typically fast when unchanged).
cargo build --release --bin nitrocop --quiet

counts=()
runtimes_ms=()

for _ in $(seq 1 "${RUNS}"); do
  tmp_output="$(mktemp)"

  start_ns="$(python3 - <<'PY'
import time
print(time.perf_counter_ns())
PY
)"

  if target/release/nitrocop "${TARGET_REPO}" >"${tmp_output}" 2>&1; then
    rc=0
  else
    rc=$?
  fi

  end_ns="$(python3 - <<'PY'
import time
print(time.perf_counter_ns())
PY
)"

  # Lint findings return 1; treat as successful benchmark execution.
  if [[ "${rc}" -gt 1 ]]; then
    echo "error: nitrocop exited with ${rc}" >&2
    tail -n 80 "${tmp_output}" >&2
    exit "${rc}"
  fi

  summary_line="$(grep -E '[0-9]+ files? inspected, [0-9]+ offenses? detected' "${tmp_output}" | tail -n 1 || true)"
  if [[ -z "${summary_line}" ]]; then
    echo "error: could not parse nitrocop summary line" >&2
    tail -n 80 "${tmp_output}" >&2
    exit 2
  fi

  offense_count="$(printf '%s' "${summary_line}" | sed -E 's/.* inspected, ([0-9]+) offenses? detected.*/\1/')"
  if ! [[ "${offense_count}" =~ ^[0-9]+$ ]]; then
    echo "error: parsed invalid offense count from summary: ${summary_line}" >&2
    exit 2
  fi

  run_ms="$(python3 - <<PY
start_ns = int(${start_ns})
end_ns = int(${end_ns})
print((end_ns - start_ns) / 1_000_000)
PY
)"

  counts+=("${offense_count}")
  runtimes_ms+=("${run_ms}")

done

nitrocop_count="${counts[0]}"
for c in "${counts[@]}"; do
  if [[ "${c}" != "${nitrocop_count}" ]]; then
    echo "error: offense count changed across repeated runs: ${counts[*]}" >&2
    exit 2
  fi
done

violation_gap=$(( nitrocop_count > RUBOCOP_TARGET ? nitrocop_count - RUBOCOP_TARGET : RUBOCOP_TARGET - nitrocop_count ))
runtimes_literal="$(IFS=,; echo "${runtimes_ms[*]}")"

runtime_ms="$(python3 - <<PY
vals = [${runtimes_literal}]
vals.sort()
mid = len(vals) // 2
if len(vals) % 2:
    med = vals[mid]
else:
    med = (vals[mid - 1] + vals[mid]) / 2
print(med)
PY
)"

runtime_over_budget_ms="$(python3 - <<PY
runtime = float(${runtime_ms})
budget = float(${RUNTIME_BUDGET_MS})
print(max(0.0, runtime - budget))
PY
)"

runtime_budget_ok="$(python3 - <<PY
runtime = float(${runtime_ms})
budget = float(${RUNTIME_BUDGET_MS})
print(1 if runtime <= budget else 0)
PY
)"

echo "METRIC violation_gap=${violation_gap}"
echo "METRIC nitrocop_violations=${nitrocop_count}"
echo "METRIC rubocop_violations=${RUBOCOP_TARGET}"
echo "METRIC runtime_ms=${runtime_ms}"
echo "METRIC runtime_over_budget_ms=${runtime_over_budget_ms}"
echo "METRIC runtime_budget_ok=${runtime_budget_ok}"
