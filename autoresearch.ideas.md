# Deferred autoresearch ideas

- Performance/StringInclude: convert literal-only regex match forms (`match/match?/=~/!~`) to `include?`/`!include?` with RuboCop-style escape interpretation (`\c`, `\C-`, escaped metachar literals) and receiver-orientation handling.
- Performance/DoubleStartEndWith: combine duplicate `start_with?`/`end_with?` predicate calls into multi-arg forms when safe, including negated `&&` forms.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
