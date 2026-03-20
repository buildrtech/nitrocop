# Cop Fix Agent Rules

You are a focused agent fixing exactly ONE cop in nitrocop, a Rust Ruby linter
that uses the Prism parser. Your complete instructions are in the task prompt
you received. Follow only those instructions.

## What you do

1. Read the task prompt — it contains the cop's source code, RuboCop reference
   implementation, test fixtures, and corpus FP/FN examples
2. Add a test case first (TDD)
3. Fix the cop's Rust source file
4. Run `cargo test --lib` with a filter for your cop only
5. Run `cargo fmt` on the file you changed
6. Add a `///` doc comment on the cop struct documenting the fix
7. Commit only your cop's files

## What files you modify

- `src/cop/<department>/<cop_name>.rs` — the cop implementation
- `tests/fixtures/cops/<department>/<cop_name>/offense.rb` — offense test cases
- `tests/fixtures/cops/<department>/<cop_name>/no_offense.rb` — clean test cases

Nothing else.

## How you test

```bash
cargo test --lib -- cop::<department>::<cop_name>
```

Use debug builds only. Do not run `cargo test` without a filter.

## How you format

```bash
cargo fmt -- src/cop/<department>/<cop_name>.rs
```

## Fixture format

Mark offenses with `^` markers on the line after the offending source:

```ruby
x = 1
     ^^ Department/CopName: Message text here.
```

## Prism parser notes

- `hash` splits into `HashNode` (literal) and `KeywordHashNode` (keyword args)
- `const` splits into `ConstantReadNode` (simple) and `ConstantPathNode` (qualified)
- `begin` splits into `BeginNode` (explicit) and `StatementsNode` (implicit body)
- `send`/`csend` merge into `CallNode` — check `.call_operator()` for `&.`
