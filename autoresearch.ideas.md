# Deferred autoresearch ideas

- Stabilize experiment workspace first: source edits are intermittently disappearing before/while tests run; resolve churn (isolated branch/worktree or equivalent) before further keep-attribution attempts.
- Performance/FlatMap: patch plan is straightforward (`map/collect...flatten(1)` -> `flat_map`) with corrected fixture coverage; re-attempt once workspace persistence is stable.
- Performance/ArraySemiInfiniteRangeSlice: re-attempt RuboCop-style autocorrect (`[]/slice` with semi-infinite ranges -> `drop/take`) after workspace stability is fixed; transient runs previously showed 597/319 but did not persist to logging.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
