# Deferred autoresearch ideas

- Performance/StringReplacement parity follow-up: add RuboCop-style autocorrect for deterministic regex-first-arg forms (currently diagnostic-only) while preserving existing metachar/flag safety guards.
- Performance/RegexpMatch parity follow-up: tighten autocorrect toward full RuboCop operator behavior (swap/`&.match?` edge forms) and add fixture-level corrected snapshots beyond current unit coverage.
- Performance/RedundantMerge parity follow-up: extend autocorrect to RuboCop-style multi-pair rewrites and modifier-form restructuring (`if/unless` wrappers).
- Performance/CaseWhenSplat parity follow-up: implement full RuboCop branch-move autocorrect (move offending `when` branches to end while preserving comments/`then` forms), beyond current in-branch condition reordering.
- Performance/Sum parity follow-up: extend autocorrect beyond no-block symbol forms to block-pass (`&:+`), block-body (`{ |acc, elem| acc + elem }`), and `map/collect...sum` rewrite paths.
- Performance/FixedSize policy/parity follow-up: decide whether to keep nitrocop-only literal-size autocorrect (RuboCop currently has no autocorrect for this cop) and, if kept, extend coverage carefully (or gate behind explicit safety config).
- Performance/MapMethodChain: retry conservative two-hop autocorrect (`base.map(&:a).map(&:b)` -> `base.map { |e| e.a.b }`) with explicit guards for inner safe-navigation (`&.`) and multi-hop chains (current attempt drifted into partial triple-chain rewrite and fixture mismatch).
- Performance/MethodObjectAsBlock parity follow-up: harden autocorrect for complex call shapes (extra positional args, receiver-qualified method objects, and alternate block-pass placements) beyond current simple baseline.
- Performance/SelectMap parity follow-up: extend autocorrect beyond symbol block-pass direct chains to block-form and block-body candidate patterns while preserving current guards for bare `select.map` enumerator and numblock/`it` semantics.
- Performance/OpenStruct parity follow-up: evaluate safer argument-bearing rewrites (`OpenStruct.new(foo: 1)`-style) or keep autocorrect limited to zero-arg constructor cases only.
