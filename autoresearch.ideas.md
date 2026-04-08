# Deferred autoresearch ideas

- Rails/WhereMissing: evaluate RuboCop-aligned autocorrect opportunities for straightforward `left_joins(...).where(...: nil)` forms to `where.missing(...)` with strict receiver/arg-shape guards.
