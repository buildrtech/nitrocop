# Deferred autoresearch ideas

- Stabilize experiment workspace first: source edits are intermittently disappearing before/while tests run; resolve churn (isolated branch/worktree or equivalent) before further keep-attribution attempts.
- Performance/ArraySemiInfiniteRangeSlice: re-attempt RuboCop-style autocorrect (`[]/slice` with semi-infinite ranges -> `drop/take`) after workspace stability is fixed; transient runs previously showed a +1 autocorrect step but did not persist to logging.
- Performance/RedundantEqualityComparisonBlock: add RuboCop-style autocorrect for `any?/all?/none?/one?` boolean comparison blocks to predicate/negated predicate forms.
