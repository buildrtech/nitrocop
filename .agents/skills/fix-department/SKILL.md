---
name: fix-department
description: Bring all cops in a gem to 100% corpus conformance with triage, TDD, and per-cop corpus verification.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Fix Department (Codex)

This skill targets one gem (for example `rubocop-performance`, `rubocop-rspec`,
or `rubocop-rails`) and drives it toward full corpus conformance (0 FP + 0 FN).

## Workflow

### Phase 0: Assess

1. Start with the scoreboard:
   ```bash
   python3 .agents/skills/fix-department/scripts/gem_progress.py --summary
   ```

2. If no gem is specified, pick one with:
   - Few diverging cops
   - Zero untested cops (preferred)
   - High adoption value

3. Run deep-dive for the selected gem:
   ```bash
   python3 .agents/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
   ```

### Phase 1: Plan Batch

Select up to 4 cops for the next batch:
- FP-only cops first (fastest safety wins)
- Then mixed FP/FN cops with highest FP
- FN-only cops last
- Skip complex Layout alignment cops until necessary

Investigate each selected cop:
```bash
python3 scripts/investigate-cop.py Department/CopName --context --fp-only --limit 10
python3 scripts/investigate-cop.py Department/CopName --context --fn-only --limit 10
```

Reduce up to 3 examples per cop:
```bash
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line --type fn
```

Read reduced repros from `/tmp/nitrocop-reduce/` and capture root-cause hypotheses.

### Phase 2: Fix Loop (Per Cop, TDD)

1. Read:
   - `src/cop/<dept>/<cop_name>.rs`
   - `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`
   - Existing investigation comments in the cop source ("Known false positives", "reverted")

2. Add failing tests first:
   - FP fix -> add case to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - FN fix -> add case to `tests/fixtures/cops/<dept>/<cop_name>/offense.rb`
   - Confirm failure:
     ```bash
     cargo test --release -p nitrocop --lib -- <cop_name_snake>
     ```

3. Implement the fix in `src/cop/<dept>/<cop_name>.rs`.

4. Re-run targeted tests and ensure they pass.

5. Verify with corpus (acceptance gate):
   ```bash
   python3 scripts/check-cop.py Department/CopName --verbose --rerun
   ```
   Corpus validation is the acceptance gate. Unit tests passing is necessary but not sufficient.

6. Handle regressions:
   - If FP increases (even with passing tests), revert the code change.
   - Add a detailed investigation comment to the cop source:
   ```rust
   /// ## Known false positives (N FP in corpus as of YYYY-MM-DD)
   ///
   /// Attempted fix: <summary> (commit XXXXXXXX, reverted).
   /// Effect: fixed A target FP but introduced B new FP (X -> Y total FP).
   /// Root cause of regression: <why this approach regressed>.
   /// A correct fix needs to: <constraints for a future correct fix>.
   ```

### Phase 3: Batch Verification

After all cops in the batch are fixed:
```bash
cargo fmt
cargo clippy --release -- -D warnings
cargo test --release
```

Re-check each fixed cop:
```bash
python3 scripts/check-cop.py Department/CopName --verbose --rerun
```

Refresh department status:
```bash
python3 .agents/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
```

Regenerate coverage artifacts after cop fixes:
```bash
cargo run --release --bin bench_nitrocop -- conform
cargo run --bin coverage_table -- --show-missing --output docs/coverage.md
```

If diverging cops remain, loop back to Phase 1.

### Phase 4: Report

Report:
- Gem targeted
- Cops fixed with FP/FN deltas
- Cops deferred/reverted and why
- Verification status (`fmt`, `clippy`, `test`, `check-cop`, coverage docs)

## Notes

- Single-agent runs: use the current working tree by default (no extra worktree required).
- Use worktrees when running in parallel with other agents, or when isolating risky experiments.
- If the user explicitly requests a worktree/branch workflow, follow that request.
- Commit each cop fix separately for safe cherry-picks.
- Never use `git stash` or `git stash pop`.
- Use local corpus files under `vendor/corpus/` when available.
- Do not copy identifiers from private repositories into source, fixtures, or commit messages.

## Arguments

- `$fix-department` — show scoreboard and choose a gem
- `$fix-department rubocop-performance` — focus that gem
- `$fix-department rubocop-rspec` — focus that gem
- `$fix-department rubocop-rails` — focus that gem
- `$fix-department --input /path/to/corpus-results.json` — use local corpus data
