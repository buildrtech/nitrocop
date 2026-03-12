---
name: fix-cops
description: Fix a batch of high-impact corpus false-positive cops for nitrocop using triage, investigation, TDD, and per-cop corpus verification.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Fix Cops (Codex)

This skill runs after a corpus oracle run. It triages the results, chooses the
highest-impact cops, and fixes them in a dedicated git worktree using TDD.
It is corpus-only: synthetic-only cops will not appear here and should be handled
via `$fix-department`.

## Workflow

### Phase 1: Triage

1. Download corpus results and rank candidates:
   ```bash
   python3 .codex/skills/triage/scripts/triage.py --fp-only --limit 20 $ARGUMENTS
   ```

2. Select up to 4 cops for this batch. Prioritize:
   - FP-only cops (FN=0)
   - Highest FP count
   - High match rate (>90%)
   - Skip complex Layout alignment cops unless necessary

3. Investigate each selected cop:
   ```bash
   python3 scripts/investigate-cop.py Department/CopName --context --fp-only --limit 10
   ```

4. Reduce up to 3 FP examples per cop to minimal reproductions:
   ```bash
   python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line
   ```
   Read reduced files from `/tmp/nitrocop-reduce/` and capture a root-cause hypothesis.

### Phase 2: Fix Each Cop (TDD)

For each selected cop, run the full loop before moving to the next one:

1. Read implementation and vendor behavior:
   - `src/cop/<dept>/<cop_name>.rs`
   - `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`
   - Check for existing investigation comments (e.g. "Known false positives",
     "reverted") in the cop source before changing logic. These document prior
     attempts that regressed on corpus; do not repeat the same approach.

2. Add failing test first:
   - FP fix: add case to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - Run targeted test and confirm it fails:
     ```bash
     cargo test --release -p nitrocop --lib -- <cop_name_snake>
     ```

3. Implement fix in `src/cop/<dept>/<cop_name>.rs`.

4. Re-run targeted test; it must pass.

5. Keep existing fixtures unless they are factually incorrect.

### Phase 3: Batch Verification

After all selected cops are fixed:

1. Run project checks:
   ```bash
   cargo fmt
   cargo clippy --release -- -D warnings
   cargo test --release
   ```

2. Verify corpus conformance for each fixed cop:
   ```bash
   python3 scripts/check-cop.py Department/CopName --verbose --rerun
   ```
   Corpus validation is the acceptance gate: unit tests passing is necessary
   but not sufficient. All fixed cops must report `PASS: aggregate offense count
   matches RuboCop for this cop`, and should also pass `verify-cop-locations.py`
   when the corpus oracle includes concrete FP/FN examples. `PASS: no new excess
   vs CI nitrocop baseline` alone is not enough. Use `--rerun` only for cops
   changed in the current batch. For untouched cops, prefer artifact mode
   (`python3 scripts/check-cop.py Department/CopName --verbose`) when the latest
   corpus oracle run is current.

3. Handle regressions: if a change increases FP count (even if unit tests pass),
   revert the code change but add a detailed investigation comment to the cop
   source file documenting:
   - What was attempted (with reverted commit SHA)
   - Exact code path changed (function/condition)
   - Acceptance-gate numbers before and after (`expected`, `actual`, `excess`, `missing`)
   - What improved and what regressed (X -> Y FP)
   - Why the regression happened
   - What a correct fix must do differently
   Use this format:
   ```rust
   /// ## Known false positives (N FP in corpus as of YYYY-MM-DD)
   ///
   /// Attempted fix: <summary> (commit XXXXXXXX, reverted).
   /// Code path changed: <file::function and condition changed>.
   /// Acceptance gate before: expected=?, actual=?, excess=?, missing=?
   /// Acceptance gate after: expected=?, actual=?, excess=?, missing=?
   /// Effect: fixed A target FP but introduced B new FP (X -> Y total FP).
   /// Root cause of regression: <why this approach overmatched or undermatched>.
   /// A correct fix needs to: <constraints for future implementation>.
   ```

4. **Document ALL investigation outcomes** as `///` comments on the cop's struct in its
   source file — not just regressions and reverts, but also cops investigated and found
   to need no fix (e.g., FPs caused by file-drop noise, config artifacts, etc.). This
   prevents future investigators from repeating the same analysis. Use:
   ```rust
   /// ## Corpus investigation (YYYY-MM-DD)
   ///
   /// Corpus oracle run #N reported FP=X, FN=Y. Investigation found ...
   /// <conclusion and whether a fix is needed>
   ```

5. Regenerate benchmark conformance artifacts only when explicitly requested by the user:
   ```bash
   cargo run --release --bin bench_nitrocop -- conform
   ```

### Phase 4: Report

Report:
- Cops fixed and their FP reductions
- Any cops deferred and why
- Verification status (`fmt`, `clippy`, `test`, `check-cop`, benchmark artifacts if requested)

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
- Treat unrelated modified files as off-limits: do not edit/revert them unless the user explicitly asks.
- Do not include unrelated files in your commit; stage only files for the cop(s) you are fixing.
- Do not pause or block on unrelated working-tree changes; continue your task and leave those files untouched.
- Commit each cop fix separately for safe cherry-picks, then integrate into `main` before ending the run.
- Never use `git stash` or `git stash pop`.
- **Do not run nitrocop or rubocop directly** — not on corpus repos, not on ad-hoc files in
  `/tmp/`, not anywhere outside the test fixtures. Running `nitrocop` on arbitrary paths fails
  ("No lockfile found") and wastes tokens. The ONLY ways to verify cop behavior are:
  - **Unit tests**: add patterns to `offense.rb` / `no_offense.rb` fixtures and run
    `cargo test --lib -- <cop_name_snake>`.
  - **Corpus validation**: `check-cop.py --rerun` for aggregate count regression on
    modified cops; `check-cop.py --verbose` (artifact mode) for untouched cops.
  - **Location validation**: `python3 scripts/verify-cop-locations.py Department/CopName`
    for modified cops when the corpus oracle includes concrete FP/FN examples.
  Never create test Ruby files outside the fixture directories.
- Do not copy identifiers from private repos into fixtures or source.
- Prefer generic minimal repros and generic naming in tests.

## Arguments

- `$fix-cops` — default top FP-only cops
- `$fix-cops --department Style` — only Style cops
- `$fix-cops --limit 10` — consider more candidates
- `$fix-cops --input /path/to/corpus-results.json` — use local file
