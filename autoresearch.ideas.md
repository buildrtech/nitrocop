# Deferred autoresearch ideas

- Performance/DoubleStartEndWith: combine duplicate `start_with?`/`end_with?` predicate calls into multi-arg forms when safe, including negated `&&` forms.
- Performance/Casecmp: rewrite downcase/upcase string comparisons to `casecmp(...).zero?` (including `!=` negation) with parentheses/safe-nav parity.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
