# Deferred autoresearch ideas

- Performance/StringReplacement parity follow-up: add RuboCop-style autocorrect for deterministic regex-first-arg forms (currently diagnostic-only) while preserving existing metachar/flag safety guards.
- Performance/RegexpMatch: add RuboCop-style autocorrect for `=~`/`!~`/`match`/`===` offenses to `match?` (with MatchData-use guards preserved).
- Performance/RedundantMerge: add conservative autocorrect for single-pair `merge!` offenses (`hash.merge!(k: v)` -> `hash[:k] = v`) before attempting multi-pair rewrites.
