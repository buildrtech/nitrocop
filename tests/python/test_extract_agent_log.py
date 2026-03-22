#!/usr/bin/env python3
"""Tests for extract_agent_log.py."""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "ci" / "extract_agent_log.py"


def run(jsonl_content: str, max_lines: int = 500) -> str:
    """Write JSONL to a temp file, run extract_agent_log.py, return stdout."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        f.write(jsonl_content)
        f.flush()
        result = subprocess.run(
            [sys.executable, str(SCRIPT), f.name, "--max-lines", str(max_lines)],
            capture_output=True, text=True,
        )
    return result.stdout


def make_event(content_blocks: list) -> str:
    """Build a JSONL line for an assistant event."""
    return json.dumps({
        "type": "assistant",
        "message": {"content": content_blocks},
    })


def test_extracts_text():
    line = make_event([{"type": "text", "text": "I found the bug."}])
    out = run(line + "\n")
    assert "I found the bug." in out


def test_extracts_bash_tool():
    line = make_event([{
        "type": "tool_use",
        "name": "Bash",
        "input": {"command": "cargo test --lib"},
    }])
    out = run(line + "\n")
    assert "`Bash`" in out
    assert "cargo test --lib" in out


def test_extracts_edit_tool():
    line = make_event([{
        "type": "tool_use",
        "name": "Edit",
        "input": {"file_path": "src/cop/style/foo.rs", "old_string": "a", "new_string": "b"},
    }])
    out = run(line + "\n")
    assert "`Edit`" in out
    assert "src/cop/style/foo.rs" in out


def test_extracts_read_tool():
    line = make_event([{
        "type": "tool_use",
        "name": "Read",
        "input": {"file_path": "src/main.rs"},
    }])
    out = run(line + "\n")
    assert "`Read`" in out
    assert "src/main.rs" in out


def test_extracts_grep_tool():
    line = make_event([{
        "type": "tool_use",
        "name": "Grep",
        "input": {"pattern": "fn check_node"},
    }])
    out = run(line + "\n")
    assert "`Grep`" in out
    assert "fn check_node" in out


def test_extracts_other_tool():
    line = make_event([{
        "type": "tool_use",
        "name": "Glob",
        "input": {"pattern": "**/*.rs"},
    }])
    out = run(line + "\n")
    assert "`Glob`" in out


def test_skips_non_assistant():
    lines = [
        json.dumps({"type": "user", "message": {"content": "fix it"}}),
        make_event([{"type": "text", "text": "On it."}]),
    ]
    out = run("\n".join(lines) + "\n")
    assert "fix it" not in out
    assert "On it." in out


def test_skips_empty_text():
    line = make_event([{"type": "text", "text": "   "}])
    out = run(line + "\n")
    assert out.strip() == ""


def test_max_lines():
    # Generate many events
    lines = []
    for i in range(100):
        lines.append(make_event([{"type": "text", "text": f"Message {i}"}]))
    out = run("\n".join(lines) + "\n", max_lines=10)
    # Should be truncated — not all 100 messages
    assert out.count("Message") < 100


def test_handles_malformed_json():
    content = "not json\n" + make_event([{"type": "text", "text": "OK"}]) + "\n"
    out = run(content)
    assert "OK" in out


def test_empty_file():
    out = run("")
    assert out.strip() == ""


def test_extracts_codex_text():
    line = json.dumps({
        "type": "response.output_item.done",
        "payload": {
            "type": "response.output_item.done",
            "item": {"content": [{"type": "text", "text": "Fixing the cop now."}]},
        },
    })
    out = run(line + "\n")
    assert "Fixing the cop now." in out


def test_extracts_codex_shell_tool():
    line = json.dumps({
        "type": "response.output_item.done",
        "payload": {
            "type": "response.output_item.done",
            "item": {"content": [{
                "type": "function_call",
                "name": "shell",
                "arguments": {"command": "cargo test --lib -- cop::style::foo"},
            }]},
        },
    })
    out = run(line + "\n")
    assert "`shell`" in out
    assert "cargo test --lib -- cop::style::foo" in out


if __name__ == "__main__":
    test_extracts_text()
    test_extracts_bash_tool()
    test_extracts_edit_tool()
    test_extracts_read_tool()
    test_extracts_grep_tool()
    test_extracts_other_tool()
    test_skips_non_assistant()
    test_skips_empty_text()
    test_max_lines()
    test_handles_malformed_json()
    test_empty_file()
    test_extracts_codex_text()
    test_extracts_codex_shell_tool()
    print("All tests passed.")
