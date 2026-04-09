# Deferred autoresearch ideas

- Performance/DeletePrefix and Performance/DeleteSuffix: deterministic method rewrite (`sub|gsub` anchored-regex + empty replacement -> `delete_prefix|delete_suffix`), mirror RuboCop literal/flag guards.
- Performance/StartWith and Performance/EndWith: convert anchored regex `match/match?/=~` calls to `start_with?`/`end_with?` with conservative escape handling.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
