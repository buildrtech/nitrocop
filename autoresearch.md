# Autoresearch: nitrocop violation-count parity vs RuboCop (wt-gph-rspec-rip-out)

## Objective
Match NitroCop's total violation count to RuboCop's total violation count on:

- Target repo: `~/Dev/wt-gph-rspec-rip-out`
- NitroCop command: `target/release/nitrocop ~/Dev/wt-gph-rspec-rip-out`
- RuboCop baseline command (run once): `cd ~/Dev/wt-gph-rspec-rip-out && rubocop`

RuboCop baseline count is fixed at **3153** offenses for this session.

## Metrics
- **Primary**: `violation_gap` (count, lower is better; **0 is exact parity**)
- **Secondary**:
  - `nitrocop_violations` (count)
  - `rubocop_violations` (count, fixed at 3153)
  - `runtime_ms` (median NitroCop runtime over repeated runs)
  - `runtime_over_budget_ms` (must stay near 0; budget is 150ms)
  - `runtime_budget_ok` (1/0 indicator for 150ms budget)

## How to Run
`./autoresearch.sh`

The script emits structured metrics as `METRIC name=value` lines.

## Files in Scope
User explicitly allowed all files in this repository, including:
- `src/**` (linter logic, cops, config resolution, output)
- `tests/**` and `tests/fixtures/**` (behavior verification)
- `bench/**` and `scripts/**` (support tooling if needed)
- Build/config files (`Cargo.toml`, `Cargo.lock`, etc.)

## Off Limits
- Do not edit `~/Dev/wt-gph-rspec-rip-out`.
- Do not re-baseline RuboCop every iteration; treat 3153 as fixed session target.

## Constraints
- Primary success condition: NitroCop offense count equals RuboCop offense count (3153).
- Secondary (monitor only): keep NitroCop runtime below 150ms when possible.
- Preserve NitroCop correctness (no intentional behavior breakage to game count).

## What's Been Tried
- 2026-03-30: Ran RuboCop once on target repo and recorded baseline `3153` offenses.
- 2026-03-30: Initial NitroCop run already reports `3153` offenses (exact count parity), runtime ~70ms (under 150ms budget).
- Immediate strategy: maintain zero-gap parity and only keep changes that preserve parity while improving/maintaining runtime.
