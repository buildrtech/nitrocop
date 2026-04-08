#!/usr/bin/env bash
set -euo pipefail

uv run python - <<'PY'
from pathlib import Path
import re

root = Path("src/cop")
exclude = {"mod.rs", "registry.rs"}

impl_pat = re.compile(r"\bimpl\s+Cop\s+for\s+")
autocorrect_pat = re.compile(r"\bfn\s+supports_autocorrect\s*\(\s*&self\s*\)\s*->\s*bool\s*\{")

cop_files = []
autocorrect_files = []

for p in root.rglob("*.rs"):
    if p.name in exclude:
        continue
    s = p.read_text()
    if impl_pat.search(s):
        cop_files.append(p)
        if autocorrect_pat.search(s):
            autocorrect_files.append(p)

missing = [p for p in cop_files if p not in autocorrect_files]

print(f"total_cops={len(cop_files)}")
print(f"with_autocorrect={len(autocorrect_files)}")
print(f"missing_autocorrect={len(missing)}")
PY
