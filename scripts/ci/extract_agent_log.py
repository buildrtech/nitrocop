#!/usr/bin/env python3
"""Extract agent conversation from a Claude Code or Codex JSONL session log.

Prints assistant text and tool call summaries as markdown.

Usage: python3 extract_agent_log.py <jsonl_path> [--max-lines N]
"""
import json
import sys
from typing import Optional


def _iter_blocks(ev: dict) -> list[dict]:
    event_type = ev.get("type")
    if event_type == "assistant":
        payload = ev.get("payload")
        if payload is not None:
            content = payload.get("content", payload.get("item", {}).get("content", []))
        else:
            content = ev.get("message", {}).get("content", [])
    elif event_type == "response.output_item.done":
        payload = ev.get("payload", {})
        content = payload.get("item", {}).get("content", payload.get("content", []))
    else:
        return []

    if isinstance(content, str):
        text = content.strip()
        return [{"type": "text", "text": text}] if text else []
    if isinstance(content, list):
        return [block for block in content if isinstance(block, dict)]
    return []


def _tool_summary(block: dict) -> Optional[str]:
    btype = block.get("type")
    if btype not in ("tool_use", "function_call"):
        return None

    name = block.get("name", block.get("function", {}).get("name", "?"))
    inp = block.get("input", block.get("arguments", {}))

    if name in ("Bash", "shell"):
        if isinstance(inp, dict):
            cmd = inp.get("command", "")
        else:
            cmd = str(inp)
        return f"> `{name}`: `{cmd[:200]}`"

    if name in ("Read", "Glob", "Grep"):
        if isinstance(inp, dict):
            arg = inp.get("file_path") or inp.get("pattern") or ""
        else:
            arg = str(inp)
        return f"> `{name}`: `{arg[:200]}`"

    if name == "Edit":
        if isinstance(inp, dict):
            fp = inp.get("file_path", "")
        else:
            fp = str(inp)
        return f"> `{name}`: `{fp}`"

    return f"> `{name}`"


def extract(path: str, max_lines: int = 500) -> None:
    lines_printed = 0
    for line in open(path):
        if lines_printed >= max_lines:
            break
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        for block in _iter_blocks(ev):
            if lines_printed >= max_lines:
                break
            if block.get("type") in ("text", "output_text") and block.get("text", "").strip():
                text = block["text"].strip()
                print(text)
                print()
                lines_printed += text.count("\n") + 2
            else:
                summary = _tool_summary(block)
                if summary is None:
                    continue
                print(summary)
                print()
                lines_printed += 2


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("path", help="Path to JSONL session log")
    parser.add_argument(
        "--max-lines", type=int, default=500, help="Max output lines"
    )
    args = parser.parse_args()
    extract(args.path, args.max_lines)
