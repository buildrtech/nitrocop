# Deferred autoresearch ideas

- Stabilize experiment workspace first: source edits are intermittently disappearing before/while tests run; resolve churn (isolated branch/worktree or equivalent) before further keep-attribution attempts.
- Performance/BigDecimalWithNumericArgument: add RuboCop-style autocorrect for float/string literal `BigDecimal(...)` and `to_d` forms, with corrected fixture coverage; latest guarded run hit confounding concurrent diffs before attribution.
- Performance/CompareWithBlock: add RuboCop-style autocorrect (`sort/min/max/minmax` comparison blocks -> `*_by` forms), including `[]` key-block rewrite cases; latest guarded attempt passed targeted tests but patch vanished before attribution.
- Performance/RedundantEqualityComparisonBlock: add RuboCop-style autocorrect for `any?/all?/none?/one?` boolean comparison blocks to predicate/negated predicate forms.
