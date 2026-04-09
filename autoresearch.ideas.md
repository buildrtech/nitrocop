# Deferred autoresearch ideas

- Performance/StringReplacement parity follow-up: add RuboCop-style autocorrect for deterministic regex-first-arg forms (currently diagnostic-only) while preserving existing metachar/flag safety guards.
- Performance/RegexpMatch: add RuboCop-style autocorrect for `=~`/`!~`/`match`/`===` offenses to `match?` (with MatchData-use guards preserved).
- Performance/RedundantMerge parity follow-up: extend autocorrect to RuboCop-style multi-pair rewrites and modifier-form restructuring (`if/unless` wrappers).
