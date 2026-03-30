# Autoresearch: Nitrocop violation-count parity with RuboCop

## Objective
Make `nitrocop` report the same offense count as RuboCop for `~/Dev/wt-gph-rspec-rip-out`.

RuboCop baseline (fixed target) is measured once and treated as constant for this session.

## Metrics
- **Primary**: `violation_gap` (count, lower is better) — absolute difference between nitrocop offenses and RuboCop baseline. Target is `0`.
- **Secondary**:
  - `nitrocop_violations` (count) — raw nitrocop offense count.
  - `nitrocop_ms` (ms) — end-to-end runtime for nitrocop command.
  - `runtime_budget_hit` (0/1) — `1` if `nitrocop_ms <= 150`, else `0`.

## How to Run
`./autoresearch.sh` — emits structured `METRIC name=value` lines.

## Files in Scope
- Entire nitrocop repository (`.`) is in scope per user instruction.
- `autoresearch.md` — session context and running notes.
- `autoresearch.sh` — benchmark harness and metric extraction.
- `autoresearch.jsonl` — experiment log.
- `autoresearch.ideas.md` — deferred promising ideas.

## Off Limits
- Do not modify `~/Dev/wt-gph-rspec-rip-out` (workload repo used for measurement).
- Do not re-run RuboCop baseline repeatedly; use fixed baseline value.

## Constraints
- RuboCop baseline offense count is fixed at `3153`.
- Keep nitrocop offense count equal to baseline (gap target = `0`).
- Secondary (non-primary) concern: keep nitrocop runtime under `150ms`.

## What's Been Tried
- Initial measurement:
  - RuboCop count confirmed once: `3153` offenses.
  - nitrocop currently reports `3153` offenses on target workload (gap `0`).
- Next focus: preserve parity while reducing runtime and avoiding parity regressions.
