#!/usr/bin/env python3
from __future__ import annotations
"""Generate a self-contained task prompt for a remote agent to fix one cop.

Produces a single markdown document containing everything the agent needs:
- Focused instructions (TDD workflow, fixture format, validation)
- The cop's Rust source
- The RuboCop Ruby implementation (ground truth)
- RuboCop spec excerpts (expect_offense / expect_no_offenses blocks)
- Current test fixtures
- Pre-computed FP/FN examples from corpus oracle

Usage:
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile --output /tmp/task.md
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile --extended
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile --input corpus-results.json
"""

import argparse
import json
import re
import sys
from pathlib import Path

# Allow importing from scripts/
SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))
from corpus_download import download_corpus_results as _download_corpus

PROJECT_ROOT = SCRIPTS_DIR.parent

# Department → vendor gem directory mapping
DEPT_TO_VENDOR = {
    "Bundler": "vendor/rubocop",
    "FactoryBot": "vendor/rubocop",
    "Gemspec": "vendor/rubocop",
    "Layout": "vendor/rubocop",
    "Lint": "vendor/rubocop",
    "Metrics": "vendor/rubocop",
    "Migration": "vendor/rubocop",
    "Naming": "vendor/rubocop",
    "Security": "vendor/rubocop",
    "Style": "vendor/rubocop",
    "Performance": "vendor/rubocop-performance",
    "Rails": "vendor/rubocop-rails",
    "RSpec": "vendor/rubocop-rspec",
    "RSpecRails": "vendor/rubocop-rspec_rails",
}

# Department name → directory name in vendor gem (usually lowercase)
DEPT_TO_DIR = {
    "FactoryBot": "factory_bot",
    "RSpec": "rspec",
    "RSpecRails": "rspec_rails",
}

# Department PascalCase → snake_case directory name in src/cop/ and tests/fixtures/
# Only needed for departments where pascal_to_snake() gives the wrong result
DEPT_TO_SRC_DIR = {
    "RSpec": "rspec",
    "RSpecRails": "rspec_rails",
    "FactoryBot": "factory_bot",
}

# Known Prism pitfalls: keyword → note
PRISM_PITFALLS = {
    "as_hash_node": (
        "hash splits into HashNode (literal `{}`) and KeywordHashNode "
        "(keyword args `foo(a: 1)`). If you handle one, check if you need the other."
    ),
    "as_constant_read_node": (
        "const splits into ConstantReadNode (simple `Foo`) and ConstantPathNode "
        "(qualified `Foo::Bar`). If you handle one, check if you need the other."
    ),
    "as_begin_node": (
        "begin is overloaded: explicit `begin..end` is BeginNode, implicit method "
        "body is StatementsNode. Check which one(s) your cop needs."
    ),
}


def pascal_to_snake(name: str) -> str:
    """Convert PascalCase to snake_case. E.g. NegatedWhile -> negated_while."""
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s)
    return s.lower()


def parse_cop_name(cop: str) -> tuple[str, str, str]:
    """Parse 'Department/CopName' into (department, cop_name, snake_name)."""
    if "/" not in cop:
        print(f"Error: cop must be in Department/CopName format, got: {cop}",
              file=sys.stderr)
        sys.exit(1)
    dept, name = cop.split("/", 1)
    return dept, name, pascal_to_snake(name)


def read_file_safe(path: Path) -> str | None:
    """Read a file, return None if it doesn't exist."""
    if path.exists():
        return path.read_text(errors="replace")
    return None


def dept_dir_name(dept: str) -> str:
    """Get the directory name for a department in src/cop/ and tests/fixtures/."""
    return DEPT_TO_SRC_DIR.get(dept, pascal_to_snake(dept))


def find_rust_source(dept: str, snake: str) -> Path:
    """Find the cop's Rust source file."""
    return PROJECT_ROOT / "src" / "cop" / dept_dir_name(dept) / f"{snake}.rs"


def find_vendor_ruby_source(dept: str, snake: str) -> Path | None:
    """Find the RuboCop Ruby implementation."""
    vendor_gem = DEPT_TO_VENDOR.get(dept)
    if not vendor_gem:
        return None
    dept_dir = DEPT_TO_DIR.get(dept, dept.lower())
    path = PROJECT_ROOT / vendor_gem / "lib" / "rubocop" / "cop" / dept_dir / f"{snake}.rb"
    return path if path.exists() else None


def find_vendor_spec(dept: str, snake: str) -> Path | None:
    """Find the RuboCop spec file."""
    vendor_gem = DEPT_TO_VENDOR.get(dept)
    if not vendor_gem:
        return None
    dept_dir = DEPT_TO_DIR.get(dept, dept.lower())
    path = PROJECT_ROOT / vendor_gem / "spec" / "rubocop" / "cop" / dept_dir / f"{snake}_spec.rb"
    return path if path.exists() else None


def extract_spec_excerpts(spec_text: str, max_blocks: int = 20) -> str:
    """Extract expect_offense and expect_no_offenses blocks from a spec file.

    Returns a condensed version with just the key test cases."""
    blocks = []
    lines = spec_text.splitlines()
    i = 0
    while i < len(lines) and len(blocks) < max_blocks:
        line = lines[i]
        # Match expect_offense or expect_no_offenses with heredoc
        if re.search(r"expect_(offense|no_offenses)\s*\(\s*<<~?\s*['\"]?(\w+)", line):
            # Find the heredoc delimiter
            m = re.search(r"<<~?\s*['\"]?(\w+)", line)
            if m:
                delimiter = m.group(1)
                block_lines = [line]
                i += 1
                while i < len(lines):
                    block_lines.append(lines[i])
                    if lines[i].strip() == delimiter:
                        break
                    i += 1
                blocks.append("\n".join(block_lines))
        # Also capture it/context/describe blocks for context
        elif re.match(r"\s*(it|context|describe)\s+['\"]", line):
            blocks.append(line)
        i += 1
    return "\n\n".join(blocks) if blocks else "(no expect_offense blocks found)"


def find_fixtures(dept: str, snake: str) -> tuple[str | None, str | None]:
    """Find offense.rb and no_offense.rb fixture files."""
    fixture_dir = PROJECT_ROOT / "tests" / "fixtures" / "cops" / dept_dir_name(dept) / snake
    offense = read_file_safe(fixture_dir / "offense.rb")
    no_offense = read_file_safe(fixture_dir / "no_offense.rb")
    # Check scenario layout
    if offense is None:
        offense_dir = fixture_dir / "offense"
        if offense_dir.is_dir():
            parts = []
            for f in sorted(offense_dir.glob("*.rb")):
                parts.append(f"# --- {f.name} ---\n{f.read_text(errors='replace')}")
            if parts:
                offense = "\n\n".join(parts)
    return offense, no_offense


def get_corpus_data(cop: str, input_path: Path | None, extended: bool) -> dict:
    """Get FP/FN data from corpus-results.json."""
    if input_path is None:
        try:
            prefer = "extended" if extended else "standard"
            input_path, _, _ = _download_corpus(prefer=prefer)
        except Exception as e:
            print(f"Warning: could not download corpus data: {e}", file=sys.stderr)
            return {"fp": 0, "fn": 0, "matches": 0, "examples": ""}

    data = json.loads(input_path.read_text())
    by_cop = data.get("by_cop", [])
    cop_entry = next((e for e in by_cop if e["cop"] == cop), None)

    if cop_entry is None:
        return {"fp": 0, "fn": 0, "matches": 0, "examples": "(cop not found in corpus data)"}

    fp = cop_entry.get("fp", 0)
    fn = cop_entry.get("fn", 0)
    matches = cop_entry.get("matches", 0)

    # Format examples
    lines = []
    fp_examples = cop_entry.get("fp_examples", [])
    fn_examples = cop_entry.get("fn_examples", [])

    if fp_examples:
        lines.append(f"### False Positives ({len(fp_examples)} examples)")
        lines.append("nitrocop flags these but RuboCop does not:\n")
        for ex in fp_examples[:30]:  # limit to 30
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- `{loc}`")
            if msg:
                lines.append(f"  Message: {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")

    if fn_examples:
        lines.append(f"\n### False Negatives ({len(fn_examples)} examples)")
        lines.append("RuboCop flags these but nitrocop does not:\n")
        for ex in fn_examples[:30]:  # limit to 30
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- `{loc}`")
            if msg:
                lines.append(f"  Message: {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")

    return {
        "fp": fp,
        "fn": fn,
        "matches": matches,
        "examples": "\n".join(lines) if lines else "(no example locations in corpus data)",
    }


def _normalize_example(ex) -> tuple[str, str, list[str] | None]:
    """Normalize an example to (loc_string, message, embedded_context)."""
    if isinstance(ex, dict):
        return ex.get("loc", ""), ex.get("msg", ""), ex.get("src")
    return ex, "", None


def detect_prism_pitfalls(rust_source: str) -> list[str]:
    """Detect relevant Prism pitfalls from the cop's Rust source."""
    notes = []
    for keyword, note in PRISM_PITFALLS.items():
        if keyword in rust_source:
            notes.append(note)
    return notes


def generate_task(
    cop: str,
    input_path: Path | None = None,
    extended: bool = False,
) -> str:
    """Generate the full task markdown for a cop."""
    dept, name, snake = parse_cop_name(cop)
    dept_snake = dept_dir_name(dept)

    # Gather all sources
    rust_path = find_rust_source(dept, snake)
    rust_source = read_file_safe(rust_path)
    if rust_source is None:
        print(f"Error: Rust source not found at {rust_path}", file=sys.stderr)
        sys.exit(1)

    ruby_path = find_vendor_ruby_source(dept, snake)
    ruby_source = read_file_safe(ruby_path) if ruby_path else None

    spec_path = find_vendor_spec(dept, snake)
    spec_source = read_file_safe(spec_path) if spec_path else None
    spec_excerpts = extract_spec_excerpts(spec_source) if spec_source else None

    offense_fixture, no_offense_fixture = find_fixtures(dept, snake)
    corpus = get_corpus_data(cop, input_path, extended)

    # Detect Prism pitfalls
    pitfalls = detect_prism_pitfalls(rust_source)

    # Build the task document
    parts = []

    # Header
    parts.append(f"# Fix {cop} — {corpus['fp']} FP, {corpus['fn']} FN\n")

    # Instructions
    focus = "FP" if corpus["fp"] > corpus["fn"] else "FN" if corpus["fn"] > corpus["fp"] else "both FP and FN"
    parts.append(f"""## Instructions

You are fixing ONE cop in **nitrocop**, a Rust Ruby linter that uses Prism for parsing.

**Current state:** {corpus['matches']:,} matches, {corpus['fp']} false positives, {corpus['fn']} false negatives.
**Focus on:** {focus} ({"nitrocop flags code RuboCop does not" if corpus["fp"] > corpus["fn"] else "RuboCop flags code nitrocop misses" if corpus["fn"] > corpus["fp"] else "both directions"}).

### Workflow
1. Read the FP/FN examples below to understand what pattern is wrong
2. Add a test case FIRST:
   - FN fix: add the missed pattern to `tests/fixtures/cops/{dept_snake}/{snake}/offense.rb` with `^` annotation
   - FP fix: add the false-positive pattern to `tests/fixtures/cops/{dept_snake}/{snake}/no_offense.rb`
3. Verify the test fails: `cargo test --lib -- cop::{dept_snake}::{snake}`
4. Fix `src/cop/{dept_snake}/{snake}.rs`
5. Verify the test passes: `cargo test --lib -- cop::{dept_snake}::{snake}`
6. Format: `cargo fmt -- src/cop/{dept_snake}/{snake}.rs`
7. Add a `///` doc comment on the cop struct documenting what you found and fixed
8. Create a branch, commit, push, and open a draft PR:
   ```
   git checkout -b fix/{dept_snake}-{snake}
   git add src/cop/{dept_snake}/{snake}.rs tests/fixtures/cops/{dept_snake}/{snake}/
   git commit -m "Fix {cop}: <one-line description of what was wrong>"
   git push -u origin fix/{dept_snake}-{snake}
   ```
   Then open a draft pull request targeting `main`.
   Do NOT push directly to `main`.

### Fixture Format
Mark offenses with `^` markers on the line AFTER the offending source line:
```
x = 1
     ^^ {cop}: Trailing whitespace detected.
```
The `^` characters must align with the offending columns. The message format is `{cop}: <message text>`.

### Rules
- Only modify files for THIS cop (source + fixtures)
- Use debug builds: `cargo test --lib -- cop::{dept_snake}::{snake}` (NOT `cargo test --release`)
- Do NOT touch unrelated files
- Do NOT use `git stash`
- Do NOT run the full test suite (`cargo test` without filter)
- This task prompt and `.kilocode/rules/cop-fix.md` contain your complete instructions.
""")

    # Prism pitfall notes
    if pitfalls:
        parts.append("### Prism Notes")
        for note in pitfalls:
            parts.append(f"- {note}")
        parts.append("")

    # Rust source
    rust_rel = rust_path.relative_to(PROJECT_ROOT)
    parts.append(f"## Current Rust Implementation\n`{rust_rel}`\n")
    parts.append(f"```rust\n{rust_source}```\n")

    # RuboCop Ruby source
    if ruby_source and ruby_path:
        ruby_rel = ruby_path.relative_to(PROJECT_ROOT)
        parts.append(f"## RuboCop Ruby Implementation (ground truth)\n`{ruby_rel}`\n")
        parts.append(f"```ruby\n{ruby_source}```\n")

    # Spec excerpts
    if spec_excerpts and spec_path:
        spec_rel = spec_path.relative_to(PROJECT_ROOT)
        parts.append(f"## RuboCop Test Excerpts\n`{spec_rel}`\n")
        parts.append(f"```ruby\n{spec_excerpts}\n```\n")

    # Fixtures
    if offense_fixture:
        parts.append(f"## Current Fixture: offense.rb\n`tests/fixtures/cops/{dept_snake}/{snake}/offense.rb`\n")
        parts.append(f"```ruby\n{offense_fixture}```\n")

    if no_offense_fixture:
        parts.append(f"## Current Fixture: no_offense.rb\n`tests/fixtures/cops/{dept_snake}/{snake}/no_offense.rb`\n")
        parts.append(f"```ruby\n{no_offense_fixture}```\n")

    # Corpus data
    parts.append("## Corpus FP/FN Examples\n")
    parts.append(corpus["examples"])
    parts.append("")

    return "\n".join(parts)


def main():
    parser = argparse.ArgumentParser(
        description="Generate a self-contained task prompt for a remote agent to fix one cop")
    parser.add_argument("cop", help="Cop name (e.g., Style/NegatedWhile)")
    parser.add_argument("--output", "-o", type=Path,
                        help="Output file path (default: stdout)")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--extended", action="store_true",
                        help="Use extended corpus (5k+ repos)")
    args = parser.parse_args()

    task = generate_task(args.cop, args.input, args.extended)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(task)
        print(f"Task written to {args.output}", file=sys.stderr)
    else:
        print(task)



if __name__ == "__main__":
    main()
