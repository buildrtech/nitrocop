#!/usr/bin/env python3
"""Tests for resolve_backend.py."""
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[2] / "scripts" / "ci"))
import resolve_backend

SCRIPT = Path(__file__).parents[2] / "scripts" / "ci" / "resolve_backend.py"


def test_all_backends_resolve():
    """Every registered backend should resolve without error."""
    for name in resolve_backend.BACKENDS:
        config = resolve_backend.resolve(name)
        assert config["cli"] in ("claude", "codex"), f"{name}: unexpected cli"
        assert config["log_format"] in ("claude", "codex"), f"{name}: unexpected log_format"
        assert config["install_cmd"], f"{name}: missing install_cmd"
        assert config["run_cmd"], f"{name}: missing run_cmd"
        assert config["log_pattern"], f"{name}: missing log_pattern"


def test_minimax_uses_claude():
    config = resolve_backend.resolve("minimax")
    assert config["cli"] == "claude"
    assert config["log_format"] == "claude"
    assert "ANTHROPIC_BASE_URL" in config["env"]


def test_claude_uses_claude():
    config = resolve_backend.resolve("claude")
    assert config["cli"] == "claude"
    assert config["log_format"] == "claude"
    assert "ANTHROPIC_BASE_URL" not in config["env"]


def test_codex_uses_codex():
    config = resolve_backend.resolve("codex")
    assert config["cli"] == "codex"
    assert config["log_format"] == "codex"
    assert "CODEX_AUTH_JSON" in config["secrets"]


def test_unknown_backend_exits():
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "unknown"],
        capture_output=True, text=True,
    )
    assert result.returncode != 0
    assert "Unknown backend" in result.stderr


def test_cli_output_format():
    """CLI should output key=value pairs."""
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "minimax"],
        capture_output=True, text=True,
    )
    assert result.returncode == 0
    lines = result.stdout.strip().split("\n")
    for line in lines:
        assert "=" in line, f"Line missing '=': {line}"
    keys = [l.split("=", 1)[0] for l in lines]
    assert "cli" in keys
    assert "install_cmd" in keys
    assert "log_format" in keys
    assert "run_cmd" in keys


def test_no_args_exits():
    result = subprocess.run(
        [sys.executable, str(SCRIPT)],
        capture_output=True, text=True,
    )
    assert result.returncode != 0


if __name__ == "__main__":
    test_all_backends_resolve()
    test_minimax_uses_claude()
    test_claude_uses_claude()
    test_codex_uses_codex()
    test_unknown_backend_exits()
    test_cli_output_format()
    test_no_args_exits()
    print("All tests passed.")
