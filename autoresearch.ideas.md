# Deferred autoresearch ideas

- Performance/StartWith and Performance/EndWith: convert anchored regex `match/match?/=~` calls to `start_with?`/`end_with?` with conservative escape handling.
- Performance/StringInclude: convert literal-only regex match forms (`match/match?/=~/!~`) to `include?`/`!include?` when receiver orientation is unambiguous.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
