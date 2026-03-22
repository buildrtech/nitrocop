#!/usr/bin/env python3
"""Tests for agent_artifacts.py."""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "agent_artifacts.py"


def render_manifest(workflow: str) -> list[str]:
    tmpdir = Path(tempfile.mkdtemp())
    output = tmpdir / "manifest.txt"
    subprocess.run(
        [sys.executable, str(SCRIPT), workflow, "--output", str(output)],
        text=True,
        capture_output=True,
        check=True,
    )
    return output.read_text().splitlines()


def test_agent_cop_fix_manifest_contains_common_and_summary_paths():
    lines = render_manifest("agent-cop-fix")
    assert "/tmp/agent.log" in lines
    assert "/tmp/git-activity/**/*" in lines
    assert "/tmp/task.md" in lines
    assert "/tmp/summary.md" in lines
    assert "/tmp/repair-summary.md" not in lines
    assert lines[-1].endswith("manifest.txt")


def test_agent_pr_repair_manifest_contains_repair_specific_paths():
    lines = render_manifest("agent-pr-repair")
    assert "/tmp/agent.log" in lines
    assert "/tmp/pr.diff" in lines
    assert "/tmp/repair-summary.md" in lines
    assert "/tmp/summary.md" not in lines
    assert lines[-1].endswith("manifest.txt")


if __name__ == "__main__":
    test_agent_cop_fix_manifest_contains_common_and_summary_paths()
    test_agent_pr_repair_manifest_contains_repair_specific_paths()
    print("All tests passed.")
