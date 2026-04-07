# Autoresearch rules: nitrocop speed on wt-gph-rspec-rip-out

## Objective
Make `nitrocop ~/Dev/wt-gph-rspec-rip-out` faster.

## Primary metric
- `total_ms` from `autoresearch.sh` (lower is better)

## Hard invariants (must never change)
- Offense count must remain exactly `3992`.
- Command behavior must remain equivalent for normal lint runs.

## Benchmark harness constraints
- Use `target/release/nitrocop`.
- Rebuild only when Rust sources are newer than binary.
- Run `--no-cache` to reduce cache artifacts.
- Run 3 samples and use median wall time.
- Parse summary line and fail run if offense count is not exactly 3992.

## Anti-overfitting guidance
- Prefer broadly useful hot-path improvements over path-specific hacks.
- Do not special-case this repository path.
- Preserve correctness checks in every experiment.
