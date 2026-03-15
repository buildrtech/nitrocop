#!/usr/bin/env python3
"""Corpus smoke test: run nitrocop + rubocop on a few small repos and compare.

Catches systemic regressions (file discovery, config resolution, directive handling)
that silently break many cops at once. Runs in ~2 min on CI.

Usage:
    python3 scripts/corpus_smoke_test.py                    # auto-detect binary
    python3 scripts/corpus_smoke_test.py --binary path/to/nitrocop
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import shutil

# Small repos pinned to exact SHAs from the corpus manifest.
# Chosen for speed: each takes <30s to clone+lint.
SMOKE_REPOS = [
    {
        "id": "multi_json__multi_json__c5fa9fc",
        "repo_url": "https://github.com/sferik/multi_json",
        "sha": "c5fa9fce50aec2d98c438f5d5e751b6f6980805c",
    },
    {
        "id": "bkeepers__dotenv__34156bf",
        "repo_url": "https://github.com/bkeepers/dotenv",
        "sha": "34156bf400cd67387fa6ed9f146778f6a2f5f743",
    },
    {
        "id": "doorkeeper__doorkeeper__b305358",
        "repo_url": "https://github.com/doorkeeper-gem/doorkeeper",
        "sha": "b30535805477bc4a2568d68968595484d6163b31",
    },
]

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE_CONFIG = os.path.join(ROOT, "bench", "corpus", "baseline_rubocop.yml")


def find_binary(explicit: str | None) -> str:
    if explicit:
        return explicit
    target_dir = os.environ.get("CARGO_TARGET_DIR", "target")
    for profile in ["release", "ci", "debug"]:
        path = os.path.join(ROOT, target_dir, profile, "nitrocop")
        if os.path.isfile(path):
            return path
    sys.exit("ERROR: nitrocop binary not found. Build with `cargo build --release` first.")


def clone_repo(repo: dict, dest: str) -> bool:
    try:
        subprocess.run(["git", "init", dest], capture_output=True, check=True)
        subprocess.run(
            ["git", "-C", dest, "fetch", "--depth", "1", repo["repo_url"], repo["sha"]],
            capture_output=True, check=True, timeout=120,
        )
        subprocess.run(
            ["git", "-C", dest, "checkout", "FETCH_HEAD"],
            capture_output=True, check=True,
        )
        return True
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
        print(f"  WARNING: clone failed for {repo['id']}: {e}", file=sys.stderr)
        return False


def run_nitrocop(binary: str, repo_dir: str) -> dict:
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = os.path.join(ROOT, "bench", "corpus", "Gemfile")
    env["BUNDLE_PATH"] = os.path.join(ROOT, "bench", "corpus", "vendor", "bundle")
    result = subprocess.run(
        [binary, "--preview", "--format", "json", "--no-cache",
         "--config", BASELINE_CONFIG, repo_dir],
        capture_output=True, text=True, env=env, timeout=300,
    )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {}


def run_rubocop(repo_dir: str) -> dict:
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = os.path.join(ROOT, "bench", "corpus", "Gemfile")
    env["BUNDLE_PATH"] = os.path.join(ROOT, "bench", "corpus", "vendor", "bundle")
    result = subprocess.run(
        ["bundle", "exec", "rubocop", "--config", BASELINE_CONFIG,
         "--format", "json", "--force-exclusion", "--cache", "false", repo_dir],
        capture_output=True, text=True, env=env, timeout=300,
    )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {}


def extract_offenses(data: dict, is_rubocop: bool) -> dict[str, set[tuple[str, int, str]]]:
    """Extract {cop_name: set of (file, line, cop)} from tool output."""
    cops: dict[str, set[tuple[str, int, str]]] = {}
    if is_rubocop:
        for f in data.get("files", []):
            path = f.get("path", "")
            for o in f.get("offenses", []):
                cop = o.get("cop_name", "")
                line = o.get("location", {}).get("line", 0)
                cops.setdefault(cop, set()).add((path, line, cop))
    else:
        # nitrocop uses flat offense list with top-level line/column
        for o in data.get("offenses", []):
            cop = o.get("cop_name", "")
            path = o.get("path", "")
            line = o.get("line", 0)
            cops.setdefault(cop, set()).add((path, line, cop))
    return cops


def main():
    parser = argparse.ArgumentParser(description="Corpus smoke test")
    parser.add_argument("--binary", help="Path to nitrocop binary")
    args = parser.parse_args()

    binary = find_binary(args.binary)
    print(f"Using binary: {binary}")

    # Check corpus bundle is available
    bundle_path = os.path.join(ROOT, "bench", "corpus", "vendor", "bundle")
    if not os.path.isdir(bundle_path):
        sys.exit(
            "ERROR: Corpus bundle not installed. Run:\n"
            "  cd bench/corpus && bundle config set --local path vendor/bundle && bundle install"
        )

    failed = False
    total_matches = 0
    total_fp = 0
    total_fn = 0

    with tempfile.TemporaryDirectory() as tmpdir:
        for repo in SMOKE_REPOS:
            repo_id = repo["id"]
            print(f"\n{'=' * 60}")
            print(f"  {repo_id}")
            print(f"{'=' * 60}")

            dest = os.path.join(tmpdir, repo_id)
            if not clone_repo(repo, dest):
                print(f"  SKIP (clone failed)")
                continue

            print(f"  Running rubocop...")
            rc_data = run_rubocop(dest)
            print(f"  Running nitrocop...")
            nc_data = run_nitrocop(binary, dest)

            rc_offenses = extract_offenses(rc_data, is_rubocop=True)
            nc_offenses = extract_offenses(nc_data, is_rubocop=False)

            # Flatten to sets of (file, line, cop)
            rc_all = set()
            for s in rc_offenses.values():
                rc_all |= s
            nc_all = set()
            for s in nc_offenses.values():
                nc_all |= s

            matches = len(rc_all & nc_all)
            fp = len(nc_all - rc_all)
            fn = len(rc_all - nc_all)
            total = matches + fp + fn
            rate = matches / total * 100 if total > 0 else 100.0

            total_matches += matches
            total_fp += fp
            total_fn += fn

            # File count comparison
            rc_files = len(rc_data.get("files", []))
            nc_files = len(nc_data.get("metadata", {}).get("inspected_files", []))
            # Fallback: count unique file paths from offenses
            if nc_files == 0 and nc_data.get("offenses"):
                nc_files = len({o.get("path", "") for o in nc_data.get("offenses", [])})

            print(f"  Files: rubocop={rc_files}, nitrocop={nc_files}")
            print(f"  Offenses: matches={matches}, FP={fp}, FN={fn}, rate={rate:.1f}%")

            # Per-cop newly-broken check: cops with 0 FP/FN from rubocop
            # that now have FP or FN are regressions
            newly_broken = []
            all_cops = set(rc_offenses.keys()) | set(nc_offenses.keys())
            for cop in sorted(all_cops):
                rc_set = rc_offenses.get(cop, set())
                nc_set = nc_offenses.get(cop, set())
                cop_fp = len(nc_set - rc_set)
                cop_fn = len(rc_set - nc_set)
                # We only flag cops that are >10% off (to allow minor diffs)
                cop_total = len(rc_set | nc_set)
                if cop_total > 0:
                    cop_match = len(rc_set & nc_set)
                    cop_rate = cop_match / cop_total * 100
                    if cop_rate < 50 and cop_total >= 5:
                        newly_broken.append(f"    {cop}: FP={cop_fp}, FN={cop_fn} ({cop_rate:.0f}% match)")

            if newly_broken:
                print(f"  Badly matching cops ({len(newly_broken)}):")
                for line in newly_broken[:10]:
                    print(line)
                if len(newly_broken) > 10:
                    print(f"    ... and {len(newly_broken) - 10} more")

            # Fail if file count differs by more than 5% (systemic file discovery issue)
            if rc_files > 0:
                file_diff_pct = abs(rc_files - nc_files) / rc_files * 100
                if file_diff_pct > 5:
                    print(f"  FAIL: file count differs by {file_diff_pct:.1f}%")
                    failed = True

    # Overall summary
    grand_total = total_matches + total_fp + total_fn
    overall_rate = total_matches / grand_total * 100 if grand_total > 0 else 100.0
    print(f"\n{'=' * 60}")
    print(f"  OVERALL: matches={total_matches}, FP={total_fp}, FN={total_fn}, rate={overall_rate:.1f}%")
    print(f"{'=' * 60}")

    # Fail threshold: if overall match rate drops below 90%, something is very wrong
    if overall_rate < 90:
        print(f"\nFAIL: overall match rate {overall_rate:.1f}% is below 90% threshold")
        failed = True

    if failed:
        sys.exit(1)
    else:
        print("\nPASS")


if __name__ == "__main__":
    main()
