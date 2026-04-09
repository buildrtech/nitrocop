# Deferred autoresearch ideas

- Performance/StringReplacement parity follow-up: add RuboCop-style autocorrect for deterministic regex-first-arg forms (currently diagnostic-only) while preserving existing metachar/flag safety guards.
- Performance/RegexpMatch parity follow-up: tighten autocorrect toward full RuboCop operator behavior (swap/`&.match?` edge forms) and add fixture-level corrected snapshots beyond current unit coverage.
- Performance/RedundantMerge parity follow-up: extend autocorrect to RuboCop-style multi-pair rewrites and modifier-form restructuring (`if/unless` wrappers).
- Performance/CaseWhenSplat parity follow-up: implement full RuboCop branch-move autocorrect (move offending `when` branches to end while preserving comments/`then` forms), beyond current in-branch condition reordering.
- Performance/Sum parity follow-up: extend autocorrect beyond no-block symbol forms to block-pass (`&:+`), block-body (`{ |acc, elem| acc + elem }`), and `map/collect...sum` rewrite paths.
- Performance/FixedSize policy/parity follow-up: decide whether to keep nitrocop-only literal-size autocorrect (RuboCop currently has no autocorrect for this cop) and, if kept, extend coverage carefully (or gate behind explicit safety config).
- Performance/MapMethodChain: retry conservative two-hop autocorrect (`base.map(&:a).map(&:b)` -> `base.map { |e| e.a.b }`) by avoiding owned/borrowed `CallNode` traversal pitfalls that caused E0308/E0382 compile failures.
- Performance/MethodObjectAsBlock: if revisiting autocorrect, replace whole call expression shape instead of only `&method(:sym)` token to avoid invalid/mismatched output in fixture checks.
