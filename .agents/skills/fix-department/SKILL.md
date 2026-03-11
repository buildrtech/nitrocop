---
name: fix-department
description: Drive a department or gem all the way to 100% conformance, including regenerated README/docs corpus artifacts and Linux CI parity.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Fix Department (Codex)

This skill targets one department or one gem (for example `Naming`,
`rubocop-performance`, `rubocop-rspec`, or `rubocop-rails`) and drives it all
the way to 100% conformance.

Final stop condition:
- The regenerated local conformance artifacts (`README.md`, `docs/corpus.md`,
  and related benchmark outputs) show 100.0% / 0 FP / 0 FN for the target.
- Linux CI/corpus oracle agrees with the local result.

Per-cop `check-cop.py --rerun` results are intermediate gates only. They are
necessary, but they are NOT sufficient to end `/fix-department`.
Run fix work from a dedicated git worktree by default.

## Workflow

### Phase 0: Assess

1. Start with the scoreboard:
   ```bash
   python3 .agents/skills/fix-department/scripts/gem_progress.py --summary
   ```
   The summary treats cops covered by the synthetic benchmark as covered, but
   synthetic-only divergence is called out in the gem deep-dive rather than the
   summary row alone.

2. Determine the target:
   - If the user specified a core `rubocop` department (for example `Naming`,
     `Layout`, `Lint`, `Metrics`, `Style`), use the owning gem only for triage,
     but treat the department row in `README.md` / `docs/corpus.md` as the real
     scorecard.
   - If the user specified a plugin gem (`rubocop-performance`,
     `rubocop-rspec`, `rubocop-rails`, etc.), the gem is both the triage scope
     and the final scorecard.

3. If no gem/department is specified, pick one with:
   - Few diverging cops
   - No uncovered cops (preferred) - `Untest` means no corpus or synthetic coverage
   - High adoption value

4. Run deep-dive for the selected gem:
   ```bash
   python3 .agents/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
   ```
   Also inspect the current checked-in scorecard row for the target in
   `README.md` and `docs/corpus.md`. That row, not the per-cop rerun output, is
   what must reach 100%.

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

**Synthetic-only cops** (zero corpus activity): If `investigate-cop.py` shows no results, the cop
only has data in the synthetic corpus. Investigate using:
```bash
# Read synthetic results for the cop
python3 -c "import json; d=json.loads(open('bench/synthetic/synthetic-results.json').read()); [print(c) for c in d['by_cop'] if c['cop']=='Department/CopName']"
# Read the synthetic source files directly (paths from fp_examples/fn_examples)
# Source files are at bench/synthetic/project/<path>
```
The synthetic corpus files at `bench/synthetic/project/` are handcrafted trigger patterns - read
the relevant source file to understand the expected behavior. Run `python3 bench/synthetic/run_synthetic.py --verbose`
to re-verify after fixing.

Reduce up to 3 examples per cop (corpus cops only - not applicable to synthetic-only cops):
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

5. Verify with the right acceptance gate for cops changed in this loop:
   ```bash
   python3 scripts/check-cop.py Department/CopName --verbose --rerun
   python3 bench/synthetic/run_synthetic.py --verbose
   ```
   Corpus validation is the intermediate acceptance gate for corpus-backed cops.
   For synthetic-only cops, rerun the synthetic benchmark and inspect that cop's
   entry in `bench/synthetic/synthetic-results.json`; `check-cop.py` has no
   signal there. Unit tests passing is necessary but not sufficient.
   Use `--rerun` only for cops whose behavior may have changed (cop source or
   related parsing/config logic). For untouched corpus cops, prefer artifact
   mode (`python3 scripts/check-cop.py Department/CopName --verbose`) when the
   latest corpus oracle run is current.

   Do not stop here. Exact-match per-cop reruns can still leave the target
   department/gem below 100% once the full corpus artifacts are regenerated.

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
   - Cops with FN from `standard:disable` handling or other config edge cases (these are still bugs to fix, not acceptable differences)

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

Re-check each fixed cop with the right acceptance gate:
```bash
python3 scripts/check-cop.py Department/CopName --verbose --rerun
python3 bench/synthetic/run_synthetic.py --verbose
```
Use `check-cop.py` for corpus-backed cops and `run_synthetic.py` for synthetic-only cops.

Refresh department/gem status:
```bash
python3 .agents/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
```

Regenerate benchmark conformance artifacts as the REQUIRED final local gate:
```bash
cargo run --release --bin bench_nitrocop -- conform
```
This is what updates `README.md`, `docs/corpus.md`, and the checked-in
conformance tables. `/fix-department` is not done until these artifacts show the
target at 100.0% / 0 FP / 0 FN.

After regenerating, inspect the target row in `README.md` and `docs/corpus.md`:
- If the row is still below 100%, loop back to Phase 1 even if the modified cops
  passed `check-cop.py --rerun`.
- If the row is 100% locally but Linux CI/corpus oracle is not yet green or
  disagrees, treat that as a parity bug and keep investigating. Do not declare
  the department/gem complete yet.
- If local and Linux CI both show 100%, the target is done.

If diverging cops remain, loop back to Phase 1.

### Phase 4: Report

Report:
- Department/gem targeted
- Cops fixed with FP/FN deltas
- Cops deferred/reverted and why
- Verification status:
  local `fmt` / `clippy` / `test`
  per-cop `check-cop` / `run_synthetic`
  regenerated `bench_nitrocop -- conform` artifact status
  `README.md` / `docs/corpus.md` target row
  Linux CI parity status (confirmed / pending / diverged)

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
- **Do not run nitrocop or rubocop directly** — not on corpus repos, not on ad-hoc files in
  `/tmp/`, not anywhere outside the test fixtures. Running `nitrocop` on arbitrary paths fails
  ("No lockfile found") and wastes tokens. The ONLY ways to verify cop behavior are:
  - **Unit tests**: add patterns to `offense.rb` / `no_offense.rb` fixtures and run
    `cargo test --lib -- <cop_name_snake>`.
  - **Corpus validation**: `check-cop.py --rerun` for modified cops; `check-cop.py --verbose`
    (artifact mode) for untouched cops.
  - **Synthetic corpus**: `python3 bench/synthetic/run_synthetic.py --verbose` for synthetic-only cops.
  Never create test Ruby files outside the fixture directories.
- `check-cop.py` is a cop-level gate, not the completion gate. Never declare
  `/fix-department` done while `README.md` / `docs/corpus.md` still show the
  target below 100%, or while Linux CI parity is still unconfirmed.
- Use local corpus files under `vendor/corpus/` when available.
- Do not copy identifiers from private repositories into source, fixtures, or commit messages.

## Arguments

- `$fix-department` — show scoreboard and choose a gem
- `$fix-department Naming` — complete the `Naming` department and keep going until its README/docs rows hit 100%
- `$fix-department rubocop-performance` — focus that gem
- `$fix-department rubocop-rspec` — focus that gem
- `$fix-department rubocop-rails` — focus that gem
- `$fix-department --input /path/to/corpus-results.json` — use local corpus data
