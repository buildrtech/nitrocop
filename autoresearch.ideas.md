# Deferred autoresearch ideas

- Performance/ArraySemiInfiniteRangeSlice: re-attempt RuboCop-style autocorrect (`[]/slice` with semi-infinite ranges -> `drop/take`) in an isolated branch/worktree; transient runs repeatedly showed 597/319 but patch state did not persist in shared-tree churn.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
