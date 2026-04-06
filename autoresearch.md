# Autoresearch: speed up nitrocop on wt-gph-rspec-rip-out

## Objective
Make `nitrocop ~/Dev/wt-gph-rspec-rip-out` faster while preserving behavior: offense count must stay exactly **3992**.
Benchmark uses the exact user command shape (default formatter, no `--format json`).

## Metrics
- **Primary**: `total_ms` (ms, lower is better)
- **Secondary**:
  - `offense_count` (must remain 3992)
  - `output_bytes` (output size sanity signal)

## How to Run
`./autoresearch.sh`

The script prints structured metrics:
- `METRIC total_ms=<number>`
- `METRIC offense_count=<number>`
- `METRIC output_bytes=<number>`

## Files in Scope
- `src/**/*.rs` — nitrocop implementation and performance changes
- `Cargo.toml` / `Cargo.lock` — only if needed for optimization
- `autoresearch.md` / `autoresearch.sh` / `autoresearch.ideas.md` — experiment control files

## Off Limits
- Corpus artifacts and benchmark cheating hacks
- Changing CLI behavior or filtering diagnostics to reduce offense count
- Any change that alters offense count from 3992 on this workload

## Constraints
- Do not overfit by special-casing this path or this repo.
- Preserve offense count = 3992.
- Prefer generally useful performance improvements.

## What's Been Tried
- Baseline pending.
