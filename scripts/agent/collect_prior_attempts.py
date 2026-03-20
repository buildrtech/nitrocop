#!/usr/bin/env python3
from __future__ import annotations
"""Collect all prior failed agent attempts for a cop.

Searches for closed/failed PRs matching the cop name, extracts their diffs
and CI failure logs, and produces a markdown summary for the retry agent.

Usage:
    python3 scripts/agent/collect_prior_attempts.py --cop Style/NegatedWhile
    python3 scripts/agent/collect_prior_attempts.py --cop Style/NegatedWhile --output /tmp/attempts.md
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path


def run_gh(args: list[str], check: bool = True) -> str:
    """Run a gh CLI command and return stdout."""
    result = subprocess.run(
        ["gh"] + args,
        capture_output=True, text=True, check=check,
    )
    return result.stdout.strip()


def find_prior_prs(cop: str) -> list[dict]:
    """Find all PRs related to this cop (open or closed)."""
    # Search by cop name in PR title — our workflow creates PRs with [Cop] prefix
    # Also search branch names like fix/style-negated_while
    query = cop.replace("/", " ")
    try:
        output = run_gh([
            "pr", "list",
            "--state", "all",
            "--search", f"{cop} in:title",
            "--json", "number,title,state,headRefName,mergedAt,closedAt,url",
            "--limit", "20",
        ])
        prs = json.loads(output) if output else []
    except subprocess.CalledProcessError:
        return []

    # Filter to PRs that look like agent attempts (not manual PRs)
    agent_prs = []
    cop_lower = cop.lower().replace("/", "")
    for pr in prs:
        title_lower = pr.get("title", "").lower().replace("/", "").replace(" ", "")
        branch = pr.get("headRefName", "").lower().replace("-", "").replace("_", "")
        if cop_lower in title_lower or cop_lower in branch:
            agent_prs.append(pr)

    return agent_prs


def get_pr_diff(pr_number: int) -> str:
    """Get the diff for a PR."""
    try:
        return run_gh(["pr", "diff", str(pr_number)], check=False)
    except Exception:
        return "(could not fetch diff)"


def get_pr_checks(pr_number: int) -> str:
    """Get CI check status for a PR."""
    try:
        return run_gh(["pr", "checks", str(pr_number)], check=False)
    except Exception:
        return "(could not fetch checks)"


def get_failed_check_logs(pr_number: int) -> str:
    """Get actual failure log output from failed CI checks."""
    try:
        # Get the checks with their details
        checks_json = run_gh([
            "pr", "checks", str(pr_number),
            "--json", "name,state,detailsUrl",
        ], check=False)
        if not checks_json:
            return ""

        checks = json.loads(checks_json)
        failed = [c for c in checks if c.get("state") in ("FAILURE", "ERROR")]
        if not failed:
            return ""

        # For each failed check, try to get the run log
        logs = []
        for check in failed[:3]:  # limit to 3 most relevant
            name = check.get("name", "unknown")
            url = check.get("detailsUrl", "")

            # Try to extract run ID from the details URL
            # Format: https://github.com/owner/repo/actions/runs/12345/job/67890
            if "/actions/runs/" in url:
                parts = url.split("/actions/runs/")[1].split("/")
                run_id = parts[0]
                try:
                    log = run_gh([
                        "run", "view", run_id,
                        "--log-failed",
                    ], check=False)
                    if log:
                        # Truncate to last 100 lines to stay within context limits
                        log_lines = log.splitlines()
                        if len(log_lines) > 100:
                            log = "\n".join(
                                ["... (truncated, showing last 100 lines) ..."]
                                + log_lines[-100:]
                            )
                        logs.append(f"#### {name} (FAILED)\n```\n{log}\n```")
                except Exception:
                    logs.append(f"#### {name} (FAILED)\nSee: {url}")
            else:
                logs.append(f"#### {name} (FAILED)\nSee: {url}")

        return "\n\n".join(logs)
    except Exception:
        return ""


def collect_attempts(cop: str) -> str:
    """Collect all prior attempts into a markdown document."""
    prs = find_prior_prs(cop)

    if not prs:
        return ""

    # Sort by PR number (chronological)
    prs.sort(key=lambda p: p.get("number", 0))

    # Filter to non-merged PRs (failed attempts) and merged ones (for context)
    failed = [p for p in prs if not p.get("mergedAt")]
    merged = [p for p in prs if p.get("mergedAt")]

    if not failed and not merged:
        return ""

    parts = []
    parts.append(f"## Prior Attempts ({len(failed)} failed, {len(merged)} merged)")
    parts.append("")

    for i, pr in enumerate(failed, 1):
        num = pr["number"]
        title = pr.get("title", "")
        state = pr.get("state", "")
        url = pr.get("url", "")

        parts.append(f"### Attempt {i}: PR #{num} ({state})")
        parts.append(f"Title: {title}")
        parts.append(f"URL: {url}")
        parts.append("")

        # Get the diff (what was tried)
        diff = get_pr_diff(num)
        if diff and diff != "(could not fetch diff)":
            # Truncate large diffs
            diff_lines = diff.splitlines()
            if len(diff_lines) > 80:
                diff = "\n".join(
                    diff_lines[:80]
                    + [f"... ({len(diff_lines) - 80} more lines truncated)"]
                )
            parts.append("#### What was changed")
            parts.append(f"```diff\n{diff}\n```")
            parts.append("")

        # Get check status
        checks = get_pr_checks(num)
        if checks:
            parts.append("#### CI Status")
            parts.append(f"```\n{checks}\n```")
            parts.append("")

        # Get actual failure logs
        logs = get_failed_check_logs(num)
        if logs:
            parts.append("#### Failure Logs")
            parts.append(logs)
            parts.append("")

    if merged:
        parts.append("### Previously Merged PRs (for reference)")
        for pr in merged:
            parts.append(f"- PR #{pr['number']}: {pr.get('title', '')} ({pr.get('url', '')})")
        parts.append("")

    return "\n".join(parts)


def main():
    parser = argparse.ArgumentParser(
        description="Collect all prior failed agent attempts for a cop")
    parser.add_argument("--cop", required=True,
                        help="Cop name (e.g., Style/NegatedWhile)")
    parser.add_argument("--output", "-o", type=Path,
                        help="Output file (default: stdout)")
    args = parser.parse_args()

    result = collect_attempts(args.cop)

    if args.output:
        args.output.write_text(result)
        if result:
            print(f"Collected prior attempts → {args.output}", file=sys.stderr)
        else:
            print(f"No prior attempts found for {args.cop}", file=sys.stderr)
    else:
        if result:
            print(result)
        else:
            print(f"No prior attempts found for {args.cop}", file=sys.stderr)


if __name__ == "__main__":
    main()
