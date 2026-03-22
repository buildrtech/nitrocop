#!/usr/bin/env python3
"""Enforce snake_case names for importable Python modules.

Top-level CLI wrappers under scripts/ are allowed to keep compatibility-oriented
names like check-cop.py. Importable modules under package-like directories must
use snake_case so they can be imported without hacks.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SNAKE_CASE_RE = re.compile(r"^[a-z][a-z0-9_]*\.py$")
PACKAGE_DIRS = [
    ROOT / "scripts" / "agent",
    ROOT / "scripts" / "ci",
    ROOT / "scripts" / "corpus",
    ROOT / "scripts" / "shared",
    ROOT / "tests" / "python",
    ROOT / "bench" / "corpus",
]
ALLOWED_WRAPPERS = {
    ROOT / "scripts" / "agent" / "generate-cop-task.py",
}


def find_violations() -> list[Path]:
    violations: list[Path] = []
    for base in PACKAGE_DIRS:
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.py")):
            if path.name == "__init__.py":
                continue
            if path in ALLOWED_WRAPPERS:
                continue
            if not SNAKE_CASE_RE.match(path.name):
                violations.append(path)
    return violations


def main() -> int:
    violations = find_violations()
    if not violations:
        print("All importable Python modules use snake_case names.")
        return 0

    print("Found non-snake_case importable Python modules:", file=sys.stderr)
    for path in violations:
        print(f"  {path.relative_to(ROOT)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
