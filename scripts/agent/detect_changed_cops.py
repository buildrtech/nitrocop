#!/usr/bin/env python3
from __future__ import annotations
"""Detect which cops changed between two git refs.

Maps changed files in src/cop/ and tests/fixtures/cops/ back to their
Department/CopName identifiers. Used by agent-cop-check.yml to know
which cops to validate.

Usage:
    python3 scripts/agent/detect_changed_cops.py --base origin/main --head HEAD
"""

import argparse
import re
import subprocess
import sys


# Department directory names that need special PascalCase handling
DEPT_OVERRIDES = {
    "rspec": "RSpec",
    "rspec_rails": "RSpecRails",
    "factory_bot": "FactoryBot",
}


def snake_to_pascal(name: str) -> str:
    """Convert snake_case to PascalCase. E.g. negated_while -> NegatedWhile."""
    return "".join(word.capitalize() for word in name.split("_"))


def dept_snake_to_pascal(name: str) -> str:
    """Convert department snake_case to PascalCase with special cases."""
    if name in DEPT_OVERRIDES:
        return DEPT_OVERRIDES[name]
    return snake_to_pascal(name)


def detect_cops(base: str, head: str) -> list[str]:
    """Find all cop names affected by changes between base and head."""
    result = subprocess.run(
        ["git", "diff", "--name-only", f"{base}...{head}"],
        capture_output=True, text=True, check=True,
    )
    changed = result.stdout.strip().splitlines()

    cops = set()
    for path in changed:
        # Match src/cop/<dept>/<name>.rs
        m = re.match(r"src/cop/([^/]+)/([^/]+)\.rs$", path)
        if m:
            dept, name = m.group(1), m.group(2)
            # Skip mod.rs and other non-cop files
            if name == "mod" or name == "node_type":
                continue
            cops.add(f"{dept_snake_to_pascal(dept)}/{snake_to_pascal(name)}")
            continue

        # Match tests/fixtures/cops/<dept>/<name>/...
        m = re.match(r"tests/fixtures/cops/([^/]+)/([^/]+)/", path)
        if m:
            dept, name = m.group(1), m.group(2)
            cops.add(f"{dept_snake_to_pascal(dept)}/{snake_to_pascal(name)}")

    return sorted(cops)


def main():
    parser = argparse.ArgumentParser(
        description="Detect which cops changed between two git refs")
    parser.add_argument("--base", default="origin/main",
                        help="Base ref (default: origin/main)")
    parser.add_argument("--head", default="HEAD",
                        help="Head ref (default: HEAD)")
    args = parser.parse_args()

    cops = detect_cops(args.base, args.head)
    for cop in cops:
        print(cop)


if __name__ == "__main__":
    main()
