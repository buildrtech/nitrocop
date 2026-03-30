# Autoresearch: close RuboCop core autocorrect gap

## Objective
Reduce the number of **implemented nitrocop cops** that are currently **not autocorrectable** even though **RuboCop core can autocorrect them**.

Current baseline from this branch:
- implemented in nitrocop: 915
- autocorrectable in nitrocop: 87
- implemented but non-autocorrectable where RuboCop core autocorrects: 372

The optimization target is to drive that 372 number down by adding correct, tested autocorrect behavior to nitrocop cops.

## Metrics
- **Primary**: `missing_core_autocorrect_cops` (count, lower is better)
- **Secondary**:
  - `nitrocop_autocorrectable_cops` (count, higher is better)
  - `implemented_core_rubocop_autocorrectable` (count, higher is better)
  - `core_overlap_autocorrectable` (count, higher is better)
  - `core_rubocop_autocorrect_total` (count, informational baseline)

## How to Run
`./autoresearch.sh`

This script computes the gap by comparing:
1. `nitrocop --list-cops`
2. `nitrocop --list-autocorrectable-cops`
3. RuboCop core autocorrectable cops from `~/Dev/rubocop` (`extend AutoCorrector` + `config/default.yml`)

It prints `METRIC ...` lines for autoresearch parsing.

## Files in Scope
- `src/cop/style/*.rs` — style cop implementations
- `src/cop/lint/*.rs` — lint cop implementations
- `src/cop/layout/*.rs` — layout cop implementations
- `src/cop/naming/*.rs` — naming cop implementations
- `src/cop/security/*.rs` — security cop implementations
- `src/cop/migration/*.rs` — migration cop implementations
- `tests/fixtures/cops/**/offense.rb` — offense fixture updates when needed
- `tests/fixtures/cops/**/no_offense.rb` — no-offense fixture updates when needed
- `tests/fixtures/cops/**/corrected.rb` — expected autocorrect output for newly autocorrectable cops
- `src/resources/autocorrect_safe_allowlist.json` — optional safe-mode allowlist updates when a cop is proven safe

## Off Limits
- Non-core plugin parity work (performance/rails/rspec) for this session
- Broad refactors unrelated to autocorrect implementation
- Manual corpus-wide conformance regeneration unless explicitly needed

## Constraints
- Follow TDD for each cop fix.
- Every real autocorrect behavior change must add/update fixtures, including `corrected.rb`.
- Keep detection behavior unchanged unless a fix is necessary for correct autocorrect behavior.
- Run targeted cop tests for changed cops.
- Keep `cargo fmt` limited to changed Rust files.
- Prefer small, incremental changes that improve the primary metric.

## What's Been Tried
- Baseline analysis complete: identified 372 implemented-but-not-autocorrectable cops where RuboCop core autocorrects.
- Gap concentration at baseline: Style (230), Lint (79), Layout (56), Naming (3), Security (3), Migration (1).
- Low-risk strategy validated: prioritize cops with one-range rewrites (keyword swaps, selector removals, whole-node rewrites).

Implemented autocorrect in this session:
- `Style/EndBlock`
- `Lint/BigDecimalNew`
- `Style/StderrPuts`
- `Style/RedundantCurrentDirectoryInPath`
- `Style/ArrayJoin`
- `Style/ArrayCoercion`
- `Style/EnvHome`
- `Lint/IdentityComparison`
- `Style/Dir`
- `Style/SymbolLiteral`
- `Style/FileNull`
- `Style/ColonMethodCall`
- `Style/ClassCheck`
- `Security/JSONLoad`
- `Lint/RegexpAsCondition`
- `Security/IoMethods`
- `Lint/TopLevelReturnWithArgument`
- `Lint/OrAssignmentToConstant`
- `Style/PercentQLiterals`
- `Style/StringMethods`
- `Style/ObjectThen`
- `Lint/ToJSON`
- `Style/BarePercentLiterals`
- `Style/PreferredHashMethods`
- `Style/ArrayIntersectWithSingleElement`
- `Style/DefWithParentheses`
- `Style/MethodDefParentheses`
- `Style/SingleArgumentDig`
- `Style/RedundantArrayConstructor`
- `Style/EmptyLambdaParameter`
- `Style/EmptyBlockParameter`
- `Style/SuperWithArgsParentheses`
- `Lint/EmptyEnsure`
- `Style/HashLookupMethod`
- `Lint/BooleanSymbol`
- `Style/RedundantPercentQ`
- `Style/QuotedSymbols`
- `Lint/UriRegexp`
- `Style/ConcatArrayLiterals`
- `Style/MinMax`
- `Lint/PercentSymbolArray`
- `Style/RedundantDoubleSplatHashBraces`
- `Style/RedundantEach`
- `Style/UnpackFirst`
- `Style/LambdaCall`
- `Style/RedundantArrayFlatten`
- `Style/MapToSet`
- `Style/InPatternThen`
- `Style/RedundantConstantBase`
- `Style/RedundantRegexpConstructor`
- `Style/CommandLiteral`
- `Style/DoubleCopDisableDirective`
- `Lint/TrailingCommaInAttributeDeclaration`
- `Style/WhileUntilDo`
- `Style/BlockComments`
- `Style/EmptyHeredoc`
- `Style/RedundantInterpolationUnfreeze`
- `Style/VariableInterpolation`
- `Lint/RescueType`
- `Style/PerlBackrefs`
- `Style/CaseEquality`
- `Style/CollectionMethods`
- `Style/RedundantStringEscape`
- `Style/RedundantRegexpEscape`
- `Style/ArrayIntersect`
- `Style/CollectionQuerying`
- `Style/CollectionCompact`
- `Style/FileRead`
- `Style/HashEachMethods`
- `Style/FileWrite`
- `Style/HashConversion`
- `Style/FileTouch`
- `Style/FileEmpty`
- `Style/GlobalStdStream`
- `Style/HashSlice`
- `Style/HashFetchChain`
- `Style/HashExcept`
- `Style/FetchEnvVar`
- `Style/ArrayFirstLast`
- `Style/NegativeArrayIndex`
- `Style/NestedFileDirname`

Current progress snapshot:
- `missing_core_autocorrect_cops`: **291** (down from 372, -81)
- `nitrocop_autocorrectable_cops`: **168** (up from 87, +81)
- Missing by department now: Style (163), Lint (67), Layout (56), Naming (3), Security (1), Migration (1)

Repeatable successful pattern:
1. Add `supports_autocorrect()`
2. Keep correction localized (selector/operator token or full-node template)
3. Add `corrected.rb`
4. Add `cop_autocorrect_fixture_tests!`
5. Run targeted `cargo test --lib -- cop::<dept>::<cop>` before measuring
