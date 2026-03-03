---
name: fix-department
description: Bring all cops in a gem to 100% corpus conformance with triage, TDD, and per-cop corpus verification.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Fix Department (Codex)

This skill targets one gem (for example `rubocop-performance`, `rubocop-rspec`,
or `rubocop-rails`) and drives it toward full corpus conformance (0 FP + 0 FN).
Run fix work from a dedicated git worktree by default.

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
   /// Code path changed: <file::function and condition changed>.
   /// Acceptance gate before: expected=?, actual=?, excess=?, missing=?
   /// Acceptance gate after: expected=?, actual=?, excess=?, missing=?
   /// Effect: fixed A target FP but introduced B new FP (X -> Y total FP).
   /// Root cause of regression: <why this approach regressed>.
   /// A correct fix needs to: <constraints for a future correct fix>.
   ```

7. **Document ALL investigation outcomes** as `///` comments on the cop's struct in its
   source file. **This is mandatory for EVERY cop in the batch** — including:
   - Cops that were **fixed** but have remaining FP/FN (document what was fixed and why
     the remaining gaps exist)
   - Cops that were **investigated but need no code fix** (e.g., FPs caused by encoding
     differences, file-drop noise, config artifacts)
   - Cops that were **deferred** (document why and what a fix would need)
   - Cops with FN from `standard:disable` or other expected behavior differences

   This prevents future investigators from repeating the same analysis. **Do not
   consider a cop "done" until it has an investigation comment, even if the fix itself
   is already committed.** Use:
   ```rust
   /// ## Corpus investigation (YYYY-MM-DD)
   ///
   /// Corpus oracle reported FP=X, FN=Y.
   ///
   /// FP=X: <what was fixed or why no fix is needed>
   /// FN=Y: <what was fixed, what remains, and root cause of any remaining FN>
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

### Phase 5: Integrate Back to Main (Default)

Do not leave retained progress only in a worktree branch.

1. Commit all progress worth keeping in the worktree:
   - Accepted cop fixes: one commit per cop (preferred).
   - Useful investigation artifacts retained in repo (for example, reverted-attempt notes): separate commit.

2. Integrate those commit(s) into `main` immediately (unless the user explicitly says not to).
   If working in a worktree, cherry-pick from the worktree branch:
   ```bash
   git checkout main
   git cherry-pick <sha1> [<sha2> ...]
   ```
   If already on main (worktree not used), commits are already there — just verify.

3. Verify integration on `main`:
   ```bash
   git log --oneline -n 10
   git status --short --branch
   ```

4. Report exactly what was integrated (commit SHA(s) and short subjects).

5. If there is truly no repo-retained progress, explicitly report that no commit was made.

## Notes

- Prefer a dedicated git worktree for code-editing runs of this skill, but worktree isolation
  may silently fail. Always verify your working directory with `git rev-parse --show-toplevel`.
  If not in a worktree, continue working on main — do not block on this.
- New worktree bootstrap (run before reducers/tests/check-cop):
  - Initialize submodules: `git submodule update --init`
  - Ensure `vendor/corpus/` exists. If the main checkout already has corpus data, symlink it into the worktree:
    `ln -s /absolute/path/to/nitrocop/vendor/corpus vendor/corpus`
  - Keep corpus wiring untracked and local-only (do not commit worktree-specific symlinks).
- Parallel-agent activity is common; expect unrelated local changes in the working tree.
- Do not revert or include unrelated files in your commit; stage only files for the cop(s) you are fixing.
- Treat unrelated modified files as off-limits: do not edit them unless the user explicitly asks.
- Do not pause or block on unrelated working-tree changes; continue your task and leave those files untouched.
- Commit each cop fix separately for safe cherry-picks, then integrate into `main` before ending the run.
- Never use `git stash` or `git stash pop`.
- **Do not run nitrocop or rubocop directly on corpus repos** — they require special env vars
  (BUNDLE_GEMFILE, BUNDLE_PATH, GIT_CEILING_DIRECTORIES) that only `check-cop.py` sets up
  correctly. Use `check-cop.py --rerun` to verify corpus behavior.
- Use local corpus files under `vendor/corpus/` when available.
- Do not copy identifiers from private repositories into source, fixtures, or commit messages.

## Arguments

- `$fix-department` — show scoreboard and choose a gem
- `$fix-department rubocop-performance` — focus that gem
- `$fix-department rubocop-rspec` — focus that gem
- `$fix-department rubocop-rails` — focus that gem
- `$fix-department --input /path/to/corpus-results.json` — use local corpus data
