#!/usr/bin/env python3
"""Generate canonical artifact path manifests for agent workflows."""

from __future__ import annotations

import argparse
from pathlib import Path

COMMON_PATHS = [
    "/tmp/agent.log",
    "/tmp/agent-events.jsonl",
    "/tmp/agent-last-message.txt",
    "/tmp/agent-result.json",
    "/tmp/agent-status.txt",
    "/tmp/agent-commits.txt",
    "/tmp/agent-recovery.stat",
    "/tmp/agent-recovery.diff",
    "/tmp/agent-recovery.patch",
    "/tmp/agent-logfile.txt",
    "/tmp/git-activity-before.json",
    "/tmp/git-activity-after.json",
    "/tmp/git-activity/**/*",
    "~/.claude/projects/**/*.jsonl",
    "~/.codex/sessions/**/*.jsonl",
]

WORKFLOW_PATHS = {
    "agent-cop-fix": [
        "/tmp/task.md",
        "/tmp/final-task.md",
        "/tmp/summary.md",
    ],
    "agent-pr-repair": [
        "/tmp/pr-diff.stat",
        "/tmp/pr.diff",
        "/tmp/final-task.md",
        "/tmp/repair.json",
        "/tmp/repair-apply.patch",
        "/tmp/repair-verify.sh",
        "/tmp/repair-verify.log",
        "/tmp/repair-summary.md",
        "/tmp/final-pr-diff.stat",
        "/tmp/final-pr.diff",
    ],
}


def manifest_for(workflow: str) -> list[str]:
    try:
        extra = WORKFLOW_PATHS[workflow]
    except KeyError as exc:
        raise SystemExit(f"unknown workflow manifest: {workflow}") from exc
    return [*COMMON_PATHS, *extra]


def write_manifest(workflow: str, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    paths = manifest_for(workflow)
    output.write_text("\n".join([*paths, str(output)]) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate agent workflow artifact manifests")
    parser.add_argument("workflow", choices=sorted(WORKFLOW_PATHS))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    write_manifest(args.workflow, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
