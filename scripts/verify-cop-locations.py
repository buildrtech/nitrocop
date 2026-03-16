#!/usr/bin/env python3
"""Per-line verification of cop FP/FN against CI corpus oracle data.

Unlike check-cop.py (aggregate counts), this checks SPECIFIC offense locations
from the CI corpus oracle. It runs nitrocop on individual corpus files and
verifies whether each known FP/FN has been fixed.

Usage:
    python3 scripts/verify-cop-locations.py Metrics/AbcSize
    python3 scripts/verify-cop-locations.py Metrics/AbcSize --fp-only
    python3 scripts/verify-cop-locations.py Metrics/AbcSize --fn-only
    python3 scripts/verify-cop-locations.py Metrics/BlockLength Metrics/MethodLength
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Optional

# Allow importing corpus_download from the same directory
sys.path.insert(0, str(Path(__file__).resolve().parent))
from corpus_download import download_corpus_results


def find_project_root() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True, text=True,
    )
    return Path(result.stdout.strip()) if result.returncode == 0 else Path(".")


def rust_build_inputs(project_root: Path) -> list[Path]:
    """Return files whose mtimes determine whether the release binary is stale."""
    paths = [
        project_root / "Cargo.toml",
        project_root / "Cargo.lock",
        project_root / "build.rs",
    ]
    src_dir = project_root / "src"
    if src_dir.is_dir():
        paths.extend(src_dir.rglob("*.rs"))
    return [path for path in paths if path.is_file()]


def stale_binary_reason(project_root: Path, nitrocop_bin: Path) -> Optional[str]:
    """Return why the release binary is stale, or None when it is fresh."""
    if not nitrocop_bin.is_file():
        return f"nitrocop binary not found at {nitrocop_bin}"

    newest_input = max(
        rust_build_inputs(project_root),
        key=lambda path: path.stat().st_mtime_ns,
        default=None,
    )
    if newest_input is None:
        return None

    binary_mtime = nitrocop_bin.stat().st_mtime_ns
    input_mtime = newest_input.stat().st_mtime_ns
    if binary_mtime >= input_mtime:
        return None

    try:
        input_label = newest_input.relative_to(project_root)
    except ValueError:
        input_label = newest_input
    return f"binary is older than {input_label}"


def ensure_fresh_release_binary(project_root: Path, nitrocop_bin: Path):
    """Build or rebuild the default release binary when missing or stale."""
    reason = stale_binary_reason(project_root, nitrocop_bin)
    if reason is None:
        return

    action = "Building" if not nitrocop_bin.exists() else "Detected stale binary"
    if nitrocop_bin.exists():
        print(f"{action} ({reason}); rebuilding with cargo build --release...", file=sys.stderr)
    else:
        print("Building nitrocop (release)...", file=sys.stderr)

    subprocess.run(
        ["cargo", "build", "--release"],
        cwd=project_root,
        check=True,
    )

    rebuilt_reason = stale_binary_reason(project_root, nitrocop_bin)
    if rebuilt_reason is not None:
        sys.exit(f"ERROR: rebuilt binary is still stale: {rebuilt_reason}")


def parse_loc(loc_str: str) -> tuple[str, str, int]:
    """Parse 'repo_id: filepath:line' into (repo_id, filepath, line)."""
    # Format: "repo_id: path/to/file.rb:123"
    repo_id, rest = loc_str.split(": ", 1)
    # Handle paths with colons by splitting from the right
    last_colon = rest.rfind(":")
    filepath = rest[:last_colon]
    line = int(rest[last_colon + 1:])
    return repo_id, filepath, line


def run_nitrocop_on_file(
    nitrocop_bin: Path, corpus_dir: Path, config_path: Path,
    repo_id: str, filepath: str, cop_name: str,
) -> set[int]:
    """Run nitrocop on a single file, return set of offense line numbers for the cop."""
    full_path = corpus_dir / repo_id / filepath
    if not full_path.exists():
        return set()

    # Build env that points bundle resolution at the corpus bundle
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(corpus_dir / "Gemfile")
    env["BUNDLE_PATH"] = str(corpus_dir / "vendor" / "bundle")

    cmd = [
        str(nitrocop_bin),
        "--only", cop_name,
        "--format", "json",
        "--no-cache",
        "--cache", "false",
        "--config", str(config_path),
        "--preview",
        str(full_path),
    ]

    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=30, env=env,
        )
    except subprocess.TimeoutExpired:
        print(f"    TIMEOUT: {filepath}", file=sys.stderr)
        return set()

    if result.returncode not in (0, 1):  # 1 = offenses found
        # Try to parse anyway, some errors still produce valid JSON
        pass

    lines = set()
    try:
        data = json.loads(result.stdout)
        for o in data.get("offenses", []):
            if o.get("cop_name") == cop_name:
                # nitrocop uses flat "line" key, not RuboCop's "location.start_line"
                line_num = o.get("line") or o.get("location", {}).get("start_line")
                if line_num is not None:
                    lines.add(line_num)
    except (json.JSONDecodeError, KeyError):
        pass

    return lines


def main():
    parser = argparse.ArgumentParser(description="Per-line FP/FN verification")
    parser.add_argument("cops", nargs="+", help="Cop names to verify")
    parser.add_argument("--fp-only", action="store_true", help="Only check FPs")
    parser.add_argument("--fn-only", action="store_true", help="Only check FNs")
    parser.add_argument("--input", type=Path, help="Path to corpus-results.json")
    args = parser.parse_args()

    project_root = find_project_root()
    corpus_dir = project_root / "vendor" / "corpus"
    config_path = project_root / "bench" / "corpus" / "baseline_rubocop.yml"
    nitrocop_bin = project_root / os.environ.get("CARGO_TARGET_DIR", "target") / "release" / "nitrocop"

    ensure_fresh_release_binary(project_root, nitrocop_bin)

    # Check corpus bundle is installed
    bundle_dir = project_root / "bench" / "corpus" / "vendor" / "bundle"
    if not bundle_dir.exists():
        print(
            "WARNING: corpus bundle not installed. Results will be wrong!\n"
            "  Fix: cd bench/corpus && BUNDLE_PATH=vendor/bundle bundle install\n",
            file=sys.stderr,
        )
    else:
        env = os.environ.copy()
        env["BUNDLE_GEMFILE"] = str(project_root / "bench" / "corpus" / "Gemfile")
        env["BUNDLE_PATH"] = str(bundle_dir)
        try:
            result = subprocess.run(
                ["bundle", "info", "--path", "rubocop"],
                capture_output=True, text=True, timeout=10,
                cwd=str(project_root / "bench" / "corpus"), env=env,
            )
            if result.returncode != 0:
                print(
                    "WARNING: corpus bundle exists but `bundle info rubocop` failed.\n"
                    f"  stderr: {result.stderr.strip()}\n"
                    "  Fix: cd bench/corpus && BUNDLE_PATH=vendor/bundle bundle install\n",
                    file=sys.stderr,
                )
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        input_path, run_id, _ = download_corpus_results()
        print(f"Using corpus oracle run {run_id}", file=sys.stderr)

    with open(input_path) as f:
        data = json.load(f)

    by_cop = {c["cop"]: c for c in data.get("by_cop", [])}

    overall_fp_fixed = 0
    overall_fp_remain = 0
    overall_fn_fixed = 0
    overall_fn_remain = 0

    for cop_name in args.cops:
        cop_data = by_cop.get(cop_name)
        if not cop_data:
            print(f"\n{cop_name}: not found in corpus results")
            continue

        fp_examples = cop_data.get("fp_examples", [])
        fn_examples = cop_data.get("fn_examples", [])

        print(f"\n{'='*70}")
        print(f"{cop_name}  (CI: FP={cop_data['fp']}, FN={cop_data['fn']})")
        print(f"{'='*70}")

        # Cache nitrocop results per (repo_id, filepath)
        nitrocop_cache: dict[tuple[str, str], set[int]] = {}

        def get_nitrocop_lines(repo_id: str, filepath: str) -> set[int]:
            key = (repo_id, filepath)
            if key not in nitrocop_cache:
                nitrocop_cache[key] = run_nitrocop_on_file(
                    nitrocop_bin, corpus_dir, config_path,
                    repo_id, filepath, cop_name,
                )
            return nitrocop_cache[key]

        # Check FPs
        if not args.fn_only and fp_examples:
            fp_fixed = 0
            fp_remain = 0
            print(f"\nFalse Positives ({len(fp_examples)} from CI):")
            for ex in fp_examples:
                loc = ex["loc"] if isinstance(ex, dict) else ex
                msg = ex.get("msg", "") if isinstance(ex, dict) else ""
                try:
                    repo_id, filepath, line = parse_loc(loc)
                except (ValueError, IndexError):
                    print(f"  ? SKIP (can't parse): {loc}")
                    continue

                nitro_lines = get_nitrocop_lines(repo_id, filepath)
                if line in nitro_lines:
                    print(f"  REMAIN  {loc}")
                    if msg:
                        print(f"          {msg}")
                    fp_remain += 1
                else:
                    print(f"  FIXED   {loc}")
                    fp_fixed += 1

            print(f"\n  FP summary: {fp_fixed} fixed, {fp_remain} remain")
            overall_fp_fixed += fp_fixed
            overall_fp_remain += fp_remain

        # Check FNs
        if not args.fp_only and fn_examples:
            fn_fixed = 0
            fn_remain = 0
            print(f"\nFalse Negatives ({len(fn_examples)} from CI):")
            for ex in fn_examples:
                loc = ex["loc"] if isinstance(ex, dict) else ex
                msg = ex.get("msg", "") if isinstance(ex, dict) else ""
                try:
                    repo_id, filepath, line = parse_loc(loc)
                except (ValueError, IndexError):
                    print(f"  ? SKIP (can't parse): {loc}")
                    continue

                nitro_lines = get_nitrocop_lines(repo_id, filepath)
                if line in nitro_lines:
                    print(f"  FIXED   {loc}")
                    fn_fixed += 1
                else:
                    print(f"  REMAIN  {loc}")
                    if msg:
                        print(f"          {msg}")
                    fn_remain += 1

            print(f"\n  FN summary: {fn_fixed} fixed, {fn_remain} remain")
            overall_fn_fixed += fn_fixed
            overall_fn_remain += fn_remain

    # Overall summary
    print(f"\n{'='*70}")
    print(f"OVERALL: FP {overall_fp_fixed} fixed / {overall_fp_remain} remain, "
          f"FN {overall_fn_fixed} fixed / {overall_fn_remain} remain")
    total_remain = overall_fp_remain + overall_fn_remain
    if total_remain == 0:
        print("ALL FP/FN VERIFIED FIXED")
    else:
        print(f"{total_remain} issues remain")
    print(f"{'='*70}")

    sys.exit(0 if total_remain == 0 else 1)


if __name__ == "__main__":
    main()
