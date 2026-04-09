# Deferred autoresearch ideas

- Performance/StringReplacement parity follow-up: add RuboCop-style autocorrect for deterministic regex-first-arg forms (currently diagnostic-only) while preserving existing metachar/flag safety guards.
- Performance/RegexpMatch parity follow-up: tighten autocorrect toward full RuboCop operator behavior (swap/`&.match?` edge forms) and add fixture-level corrected snapshots beyond current unit coverage.
- Performance/RedundantMerge parity follow-up: extend autocorrect to RuboCop-style multi-pair rewrites and modifier-form restructuring (`if/unless` wrappers).
- Performance/CaseWhenSplat parity follow-up: implement full RuboCop branch-move autocorrect (move offending `when` branches to end while preserving comments/`then` forms), beyond current in-branch condition reordering.
- Performance/Sum parity follow-up: extend autocorrect beyond no-block symbol forms to block-pass (`&:+`), block-body (`{ |acc, elem| acc + elem }`), and `map/collect...sum` rewrite paths.
