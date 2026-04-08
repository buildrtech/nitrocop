# Deferred autoresearch ideas

- Rails/WhereRange: evaluate conservative RuboCop-aligned autocorrect for deterministic `where("col >= ? AND col <= ?", ...)` shapes to range-hash form with strict SQL-shape guards.
