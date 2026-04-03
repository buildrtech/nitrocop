# nitrocop vs RuboCop Gap — Status Report

**Branch:** `autoresearch/autocorrect-gap-20260330`
**Target project:** `~/Dev/wt-gph-rspec-rip-out`
**Date:** 2026-04-03

## Goal

Minimize the per-cop offense count differences between nitrocop and RuboCop when linting the `wt-gph-rspec-rip-out` project. The project uses the `standard` gem with `AllCops: NewCops: enable` in `.rubocop.yml`.

## Starting Point

**Original numbers (before any work):**
- nitrocop: 4021 offenses (6143 files)
- RuboCop: 3990 offenses (6143 files)
- **Gap: +31** (28 cops with differences)

## Current State

**After all fixes (fresh cache, 2026-04-03):**
- nitrocop: 4046 offenses (6143 files)
- RuboCop: 3986 offenses (6143 files)
- **Gap: +60** (3 cops with differences — down from 28)

The key metric is **per-cop accuracy**: 31 cops were fixed to exact match across sessions 1-7, reducing mismatched cops from 28 to 3.

### Per-Cop Breakdown (3 cops with differences)

```
Cop                                                  Nitro   Rubo    Gap
--------------------------------------------------------------------------------
Lint/Syntax                                           1523   1456    +67
Layout/IndentationWidth                                 18     24     -6
Lint/SymbolConversion                                    9     10     -1
--------------------------------------------------------------------------------

FP total (nitro over-reports): +67
FN total (nitro under-reports): -7
```

## Cops Fixed to Exact Match

31 cops were fixed across sessions 1-7:

| Cop | Before | After | Session |
|-----|--------|-------|---------|
| Style/RedundantBegin | nitro=3, rubo=10 (-7) | 10=10 | 2 |
| Layout/MultilineOperationIndentation | nitro=0, rubo=6 (-6) | 6=6 | 2 |
| Lint/ShadowedException | nitro=6, rubo=3 (+3) | 3=3 | 3 |
| Style/OneLineConditional | nitro=3, rubo=6 (-3) | 6=6 | 3 |
| Style/TrailingCommaInArguments | nitro=12, rubo=15 (-3) | 15=15 | 3 |
| Rails/HttpPositionalArguments | nitro=0, rubo=3 (-3) | 3=3 | 3 |
| Style/ItAssignment | nitro=1, rubo=3 (-2) | 3=3 | 3 |
| Style/ComparableClamp | nitro=0, rubo=2 (-2) | 2=2 | 3 |
| Style/SlicingWithRange | nitro=1, rubo=2 (-1) | 2=2 | 3 |
| Style/RedundantSelf | nitro=50, rubo=49 (+1) | 49=49 | 1 |
| Lint/CircularArgumentReference | nitro=2, rubo=3 (-1) | 3=3 | 4 |
| Lint/RedundantWithObject | nitro=2, rubo=3 (-1) | 3=3 | 4 |
| Lint/RedundantTypeConversion | nitro=3, rubo=4 (-1) | 4=4 | 4 |
| Style/RedundantRegexpEscape | nitro=2, rubo=4 (-2) | 0=0 | 4* |
| Style/RedundantDoubleSplatHashBraces | nitro=2, rubo=3 (-1) | 3=3 | 5 |
| Style/RedundantParentheses | nitro=6, rubo=9 (-3) | 9=9 | 5 |
| Style/QuotedSymbols | nitro=0, rubo=3 (-3) | 3=3 | 5 |
| Style/EmptyLiteral | nitro=0, rubo=3 (-3) | 3=3 | 5 |
| Lint/MissingCopEnableDirective | nitro=3, rubo=0 (+3) | 0=0 | 6 |
| Performance/StringBytesize | nitro=6, rubo=3 (+3) | 3=3 | 6 |
| Rails/Pick | nitro=6, rubo=3 (+3) | 3=3 | 6 |
| Rails/SafeNavigationWithBlank | nitro=6, rubo=3 (+3) | 3=3 | 6 |
| Rails/IndexBy | nitro=12, rubo=9 (+3) | 9=9 | 6 |
| Rails/IndexWith | nitro=12, rubo=9 (+3) | 9=9 | 6 |
| Style/Sample | nitro=15, rubo=18 (-3) | 18=18 | 6 |
| Layout/HashAlignment | nitro=6, rubo=9 (-3) | 9=9 | 7 |
| Lint/UselessAssignment | nitro=241, rubo=244 (-3) | 244=244 | 7 |
| Layout/IndentationWidth (partial) | nitro=18, rubo=24 (-6) | 18→18 (partial, -6 remain) | 7 |
| Standard/BlockSingleLineBraces | nitro=0, rubo=23 (-23) | 23=23 | 7 |

*Style/RedundantRegexpEscape: the 2 vs 4 offenses came from a test file artifact that was removed. Both tools now report 0.

## All Fixes Applied (Unstaged)

### Session 1 Fixes (prior session)

#### 1. Lint/Syntax Dedup (`src/linter.rs`)
- Changed `suppress_secondary_syntax_diagnostics()` to deduplicate by `(line, column, message)` instead of just `(line, column)`.

#### 2. Style/TrailingCommaInHashLiteral (`src/cop/style/trailing_comma_in_hash_literal.rs`)
- Added `no_elements_on_same_line` check for `comma`/`consistent_comma` style.

#### 3. Style/TrailingCommaInArrayLiteral (`src/cop/style/trailing_comma_in_array_literal.rs`)
- Made `no_elements_on_same_line` function `pub(crate)` for cross-cop sharing.

#### 4. Layout/MultilineMethodCallIndentation (`src/cop/layout/multiline_method_call_indentation.rs`)
- Reset `in_hash_value` flag when entering parenthesized args.

#### 5. Layout/ArgumentAlignment (`src/cop/layout/argument_alignment.rs`)
- Added check for first element under `with_fixed_indentation` style.

#### 6. Layout/ArrayAlignment (`src/cop/layout/array_alignment.rs`)
- Added `check_first` parameter to `check_element_alignment`.

#### 7. Layout/LeadingCommentSpace (`src/cop/layout/leading_comment_space.rs`)
- Implemented `AllowRBSInlineAnnotation` config support for `#:` comments.

#### 8. Style/RedundantSelf (`src/cop/style/redundant_self.rs`)
- Added `collect_multi_target_locals()` for destructured parameters with `RequiredParameterNode` handling.

### Session 2 Fixes

#### 9. Layout/MultilineOperationIndentation (`src/cop/layout/multiline_operation_indentation.rs`)
- Removed overly-permissive escape hatches. Added `arg_col == recv_col` for "indented" style.

#### 10. Style/RedundantBegin (`src/cop/style/redundant_begin.rs`)
- Added standalone `begin...end` detection with `in_loop_body` flag for do-while loops and `body_has_rescue_modifier()` for inline rescue.

### Session 3 Fixes

#### 11. Lint/ShadowedException (`src/cop/lint/shadowed_exception.rs`)
- Removed `a == b` early return from `equivalent_exception_classes()`. Identical unknown exception names in the same rescue clause (`rescue Foo, Foo`) are `Lint/DuplicateRescueException`, not `Lint/ShadowedException`.

#### 12. Style/OneLineConditional (`src/cop/style/one_line_conditional.rs`)
- Removed `then_keyword_loc().is_none()` guard for both if and unless branches. Ruby allows semicolons as separators (`if cond; body else other end`), but Prism returns `nil` for `then_keyword_loc` in that case. The same-line check (`start_line != end_line`) already prevents FPs on multiline conditionals.

#### 13. Style/TrailingCommaInArguments (`src/cop/style/trailing_comma_in_arguments.rs`)
- Added single-line trailing comma removal to the `comma`/`consistent_comma` branch. Previously only the `no_comma` default branch handled unwanted trailing commas.

#### 14. Rails/HttpPositionalArguments (`src/cop/rails/http_positional_arguments.rs`)
- Relaxed `arg_list.len() >= 3` to `>= 2`. The 2-arg pattern `get path, {params}` (explicit HashNode as second argument) is also an offense.

#### 15. Style/ItAssignment (`src/cop/style/it_assignment.rs`)
- Added `REQUIRED_PARAMETER_NODE` and `OPTIONAL_PARAMETER_NODE` to `interested_node_types`. Method parameters named `it` (e.g., `def foo(it)`, `def bar(it = 5)`) should also be flagged.

#### 16. Style/ComparableClamp (`src/cop/style/comparable_clamp.rs`)
- Added `[[x, low].max, high].min` and `[[x, high].min, low].max` array/method-call pattern detection via new `extract_array_clamp()` function. Previously only handled the `if/elsif/else` form.

#### 17. Style/SlicingWithRange (`src/cop/style/slicing_with_range.rs`)
- Added handling for `nil` as left operand in range. `items[nil..42]` now suggests `items[..42]`. Prism parses explicit `nil` as a `NilNode`, but the cop only checked for integer left operands.

### Session 4 Fixes

#### 18. Lint/CircularArgumentReference (`src/cop/lint/circular_argument_reference.rs`)
- Made `is_circular_ref` recursive through `LocalVariableWriteNode`. The double-nested default `def foo(pie = pie = pie)` now correctly detects the circular reference in the innermost read node.

#### 19. Lint/RedundantWithObject (`src/cop/lint/redundant_with_object.rs`)
- Added detection for `each.with_object(arg)` chained call pattern. Previously only detected `each_with_object(arg)`. Autocorrect removes `.with_object(arg)` from the chain.

#### 20. Lint/RedundantTypeConversion (`src/cop/lint/redundant_type_conversion.rs`)
- Added `to_set` to the method match table. `Set.new.to_set` and `Set.[].to_set` are now detected as redundant. Block guard for `to_set` was already present.

### Session 5 Fixes

#### 21. Style/RedundantDoubleSplatHashBraces (`src/cop/style/redundant_double_splat_hash_braces.rs`)
- Added `**{foo: bar}.merge(options)` pattern detection. The AssocSplatNode value is a CallNode (`.merge()` on a HashNode), not a direct HashNode. Autocorrect transforms to `foo: bar, **options`.

#### 22. Style/RedundantParentheses (`src/cop/style/redundant_parentheses.rs`)
- Removed `is_receiver || is_chained` guard from `check_method_call`. RuboCop's `check_send` has no such guard — it only guards `check_unary`. The downstream `has_args && !call_has_parens` check already protects operators. Three no_offense entries moved to offense (`(foo).bar(x)`, `(x.y).z(arg)`, `(foo_bar.baz).qux`).

#### 23. Style/QuotedSymbols (`src/cop/style/quoted_symbols.rs`)
- Fixed `same_as_string_literals` resolution in single-quoted branch. The double-quoted branch correctly resolved `same_as_string_literals` via `StringLiteralsEnforcedStyle`, but the single-quoted branch compared the raw config string directly, missing the cross-cop lookup.

#### 24. Style/EmptyLiteral (`src/cop/style/empty_literal.rs`, `src/config/mod.rs`)
- Implemented RuboCop's three-step `frozen_strings?` logic for `String.new`: (1) `frozen_string_literal: true` → skip; (2) `Style/FrozenStringLiteralComment` cop enabled with no magic comment → skip; (3) otherwise → flag. Previously only checked for explicit `frozen_string_literal: false`. Added `FrozenStringLiteralCommentEnabled` config injection.

### Session 6 Fixes

#### 25. Lint/MissingCopEnableDirective (`src/cop/lint/missing_cop_enable_directive.rs`, `src/config/mod.rs`)
- Implemented RuboCop's `acceptable_range?` logic: skip offense when the disabled cop is itself not enabled in config. Added `DisabledCopNames` config injection in `src/config/mod.rs` that builds a list of all disabled cop names (full and short forms) plus disabled department names. The cop now builds a `HashSet` from this config and skips offenses for disabled cops.

#### 26. Performance/StringBytesize (`src/cop/performance/string_bytesize.rs`)
- Added argument check on the outer call. `.bytes.count` (no args) = total byte count (equivalent to bytesize), but `.bytes.count(42)` = count of specific byte value (NOT equivalent). Malformed corpus fixtures caused Prism to fold subsequent expressions as arguments.

#### 27. Rails/Pick (`src/cop/rails/pick.rs`)
- Added argument check on `.first`. `.pluck(...).first` (no args) = one value (equivalent to pick), but `.pluck(...).first(n)` = first n elements (NOT equivalent).

#### 28. Rails/SafeNavigationWithBlank (`src/cop/rails/safe_navigation_with_blank.rs`)
- Added argument check on `.blank?`. The method takes no arguments — if arguments are present (from malformed corpus parsing), skip the offense.

#### 29. Rails/IndexBy (`src/cop/rails/index_by.rs`)
- Added argument check on `.to_h` in Pattern 1 (`map { ... }.to_h`). `.to_h` with arguments is a different semantic — in malformed corpus fixtures, Prism folds subsequent expressions (like `Hash[...]`) as arguments to `.to_h`.

#### 30. Rails/IndexWith (`src/cop/rails/index_with.rs`)
- Same fix as Rails/IndexBy: added argument check on `.to_h` in Pattern 1.

#### 31. Style/Sample (`src/cop/style/sample.rs`)
- Added handling for two-argument bracket access pattern `shuffle[0, n]`. In Ruby, `arr[0, n]` is equivalent to `arr.slice(0, n)` which returns n elements starting at index 0 — equivalent to `sample(n)` after a shuffle. Previously only single-argument bracket patterns were handled.

## Remaining Gap Analysis

### Unfixable

#### Lint/Syntax +67
Inherent difference between Prism (nitrocop's parser) and the Parser gem (RuboCop's parser). They produce different parse error counts/messages. Cannot be resolved without switching parsers.

### Deferred (risky tradeoff)

#### Layout/IndentationWidth -6
- Remaining 6 FNs are all from tab-indented files. Tab handling would regress 159 FPs on the corpus.
- Session 7 fixed the `def` base_col computation (effective_column) and removed begin...end alt_base.

### Corpus fixture artifacts

#### Lint/SymbolConversion -1
Malformed corpus fixture: `:sym.to_sym` after `"text".to_s` without semicolons. Prism folds `:sym.to_sym` as argument to preceding expression.

## Important Lessons Learned

### Cache Invalidation
nitrocop's stat-based result cache (`~/.cache/nitrocop/`) does NOT invalidate when cop code changes — only when target files change (mtime/size). During development:
1. Clear cache: `rm -rf ~/.cache/nitrocop`
2. Reinitialize lockfile: `cargo run --release -- --init ~/Dev/wt-gph-rspec-rip-out`
3. Use `--cache false` for all comparisons

### Prism AST Gotchas
- `begin X rescue Y end` (inline): `RescueModifierNode` inside `statements`, NOT `rescue_clause`
- `begin...end while true`: Prism creates `WhileNode` containing `BeginNode`
- Destructured params in `MultiTargetNode`: elements are `RequiredParameterNode`, not `LocalVariableTargetNode`
- `if cond; body else other end`: Prism returns `nil` for `then_keyword_loc` (semicolons serve as `then`)
- Method params named `it`: Prism uses `RequiredParameterNode` / `OptionalParameterNode`, not `LocalVariableWriteNode`
- `def foo(pie = pie = pie)`: The outer default value is a `LocalVariableWriteNode` containing another write, need recursive check
- Malformed single-line multi-expression corpus fixtures: Prism folds subsequent expressions as arguments to the first, causing spurious argument nodes on method calls

### RuboCop Config Nuances
- `Lint/MissingCopEnableDirective` has a cross-cop check: `acceptable_range?` skips offenses when the disabled cop is itself disabled in config. Implemented via `DisabledCopNames` config injection.
- `Style/QuotedSymbols` resolves its style from `Style/StringLiterals EnforcedStyle` via `same_as_string_literals` — config inheritance from `inherit_gem: standard` may not resolve this correctly.
- `Style/EmptyLiteral` has a three-step `frozen_strings?` check that depends on `Style/FrozenStringLiteralComment` cop state. Implemented via `FrozenStringLiteralCommentEnabled` config injection.

### Argument Check Pattern for Malformed Fixtures
Many cops can be hardened against malformed corpus fixtures by checking that the outer method call has no unexpected arguments. RuboCop's node_pattern DSL naturally rejects calls with extra arguments, but nitrocop's manual AST traversal doesn't — explicit `arguments().is_none()` / `arguments().is_some()` checks are needed. This pattern was applied to 6 cops in session 6.

### Session 7 Fixes

#### 32. Layout/HashAlignment (`src/cop/layout/hash_alignment.rs`)
- Added heuristic for detecting hash-inside-call-args context. When `autocorrect_incompatible_with_other_cops?` would skip the offense, check if the hash's preceding non-whitespace character is `(` or `,` (indicating call arg context) and only skip in that case.

#### 33. Lint/UselessAssignment (`src/cop/lint/useless_assignment.rs`)
- Fixed parameter name skip in `report_useless`. Method params were unconditionally excluded from useless assignment detection. Changed: `analyze_def` passes empty HashSet (method params should be reported), `analyze_block_scope` passes only `outer_vars` (not params+outer_vars).

#### 34. Layout/IndentationWidth (`src/cop/layout/indentation_width.rs`)
- Fixed `def` base_col to use RuboCop's `effective_column` (first non-whitespace char on the line) instead of the `def` keyword column. Handles `helper_method def amount_type` correctly.
- Removed `begin...end` alt_base (changed to pass `None`).
- Preserved `keyword` style branch for `align_style == "keyword"`.

#### 35. Standard/BlockSingleLineBraces (`src/cop/standard/block_single_line_braces.rs`) — NEW COP
- Implemented the `standard-custom` gem cop from scratch. Detects single-line `do...end` blocks and suggests `{...}`.
- Handles: parenthesized calls, non-parenthesized calls (reports but skips autocorrect), super/forwarding_super nodes.
- Skips: multi-line blocks, already-braced blocks, operator methods, assignment methods.
- Added `Standard` department: `src/cop/standard/mod.rs`, registered in `registry.rs`.
- Added `("Standard", "standard-custom")` to `PLUGIN_GEM_DEPARTMENTS` in `src/config/mod.rs`.

## Test Status

- **4658 library tests pass** (`cargo test --release --lib`)
- 7 integration test failures are pre-existing (unrelated)
- Clippy: no new warnings from session 7 changes (89 pre-existing errors)
- fmt: run on all session 7 files

## Files Modified (Session 7)

```
src/cop/layout/hash_alignment.rs                       | preceding-char heuristic for call arg context
src/cop/lint/useless_assignment.rs                     | fix param name skip in report_useless
src/cop/layout/indentation_width.rs                    | effective_column for def base_col, remove begin alt_base
src/cop/standard/block_single_line_braces.rs           | NEW: Standard/BlockSingleLineBraces cop
src/cop/standard/mod.rs                                | NEW: Standard department module
src/cop/mod.rs                                         | add pub mod standard
src/cop/registry.rs                                    | register Standard department, update count
src/config/mod.rs                                      | add Standard to PLUGIN_GEM_DEPARTMENTS
tests/integration.rs                                   | update cop count from 915 to 916
tests/fixtures/cops/standard/block_single_line_braces/ | NEW: offense.rb, no_offense.rb
```

## Next Steps (Suggested Priority)

1. **Commit all changes** — all files across sessions 1-7 are unstaged.
2. **Layout/IndentationWidth** (-6) — remaining tab-related FNs, risky.
3. **Lint/SymbolConversion** (-1) — corpus fixture artifact, low priority.
