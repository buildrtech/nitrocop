# Deferred autoresearch ideas

- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
- Rails/IndexBy or Rails/IndexWith: evaluate selector-level autocorrect opportunities with strict shape guards to avoid semantic drift.
