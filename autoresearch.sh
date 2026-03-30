#!/usr/bin/env bash
set -euo pipefail

RUBOCOP_DIR="${RUBOCOP_DIR:-$HOME/Dev/rubocop}"
if [[ ! -d "$RUBOCOP_DIR" ]]; then
  echo "error: rubocop directory not found: $RUBOCOP_DIR" >&2
  exit 1
fi

tmp_all=$(mktemp)
tmp_auto=$(mktemp)
cleanup() {
  rm -f "$tmp_all" "$tmp_auto"
}
trap cleanup EXIT

# Pre-checks: ensure nitrocop CLI paths still work and compile.
cargo run --quiet -- --list-cops > "$tmp_all"
cargo run --quiet -- --list-autocorrectable-cops > "$tmp_auto"

python3 - "$tmp_all" "$tmp_auto" "$RUBOCOP_DIR" <<'PY'
import re
import sys
from pathlib import Path

all_path = Path(sys.argv[1])
auto_path = Path(sys.argv[2])
rubocop_dir = Path(sys.argv[3])

nitro_all = {line.strip() for line in all_path.read_text().splitlines() if line.strip()}
nitro_auto = {line.strip() for line in auto_path.read_text().splitlines() if line.strip()}

config_path = rubocop_dir / "config" / "default.yml"
rubocop_cop_root = rubocop_dir / "lib" / "rubocop" / "cop"

if not config_path.exists() or not rubocop_cop_root.exists():
    print(f"error: invalid rubocop tree at {rubocop_dir}", file=sys.stderr)
    sys.exit(1)

# Parse canonical cop names from default.yml (e.g., Style/AndOr)
key_pattern = re.compile(r'^([A-Z][A-Za-z0-9]*/[A-Za-z0-9]+):\s*$')
keys = []
for line in config_path.read_text().splitlines():
    m = key_pattern.match(line)
    if m:
        keys.append(m.group(1))

# Normalize filename-derived cop IDs to config keys
norm_to_key = {}
for key in keys:
    dept, cop = key.split('/')
    norm = f"{dept.lower()}/{re.sub(r'[^a-z0-9]', '', cop.lower())}"
    norm_to_key[norm] = key

# RuboCop core autocorrectable set: cops with `extend AutoCorrector`
rubo_auto = set()
for file in rubocop_cop_root.rglob('*.rb'):
    rel = file.relative_to(rubocop_cop_root).as_posix()
    if rel.startswith('internal_affairs/'):
        continue
    if rel.endswith('/base.rb') or rel.endswith('/cop.rb'):
        continue

    text = file.read_text(encoding='utf-8', errors='ignore')
    if 'extend AutoCorrector' not in text:
        continue

    parts = Path(rel).parts
    if len(parts) < 2:
        continue

    dept = parts[0]
    stem = Path(parts[-1]).stem
    norm = f"{dept.lower()}/{re.sub(r'[^a-z0-9]', '', stem.lower())}"
    key = norm_to_key.get(norm)
    if key:
        rubo_auto.add(key)

missing = sorted((nitro_all - nitro_auto) & rubo_auto)
implemented_core_rubocop_auto = nitro_all & rubo_auto
overlap_auto = nitro_auto & rubo_auto

print(f"METRIC missing_core_autocorrect_cops={len(missing)}")
print(f"METRIC nitrocop_autocorrectable_cops={len(nitro_auto)}")
print(f"METRIC implemented_core_rubocop_autocorrectable={len(implemented_core_rubocop_auto)}")
print(f"METRIC core_overlap_autocorrectable={len(overlap_auto)}")
print(f"METRIC core_rubocop_autocorrect_total={len(rubo_auto)}")

# Helpful, non-metric context for iteration planning
from collections import Counter
by_dept = Counter(name.split('/')[0] for name in missing)
print("Top missing departments:")
for dept, count in sorted(by_dept.items(), key=lambda kv: (-kv[1], kv[0])):
    print(f"  {dept}: {count}")
PY
