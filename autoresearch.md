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

## What's been tried (latest)

### Kept wins
- Persistent lockfile-stamped gem-path cache in `src/config/gem_path.rs` (including negative cache for missing gems) removed Bundler startup from steady runs.
- `Rails/WhereEquals`: moved SQL regex compilation to `LazyLock` statics and removed transient arg-list allocations.
- `Rails/WhereMissing`: added early outer-call name gating and removed temporary vector allocation in `left_joins` argument parsing.
- `Rails/DynamicFindBy`: moved config option parsing behind `find_by_` name gating and removed per-call argument vec allocation.
- `Lint/MissingCopEnableDirective`: fast `rubocop:` byte-level reject path + reduced per-file allocation churn.

### Rejected directions
- Broad linter dispatch capability refactor (`uses_check_lines`/`uses_check_source` caching + node-cop skipping) regressed.
- Small `SpaceAroundOperators` micro-optimization (avoid clone of reported offsets) showed no measurable gain.
- Conditional `SpaceAroundKeyword` end-skip collector gating regressed.

### Current hotspot direction
- Remaining heavy source cops are primarily `Layout/SpaceAroundOperators`, `Layout/SpaceAroundKeyword`, and `Lint/UselessAssignment`; next attempts should focus on large allocation/branch-elimination wins, not tiny micro-edits.
