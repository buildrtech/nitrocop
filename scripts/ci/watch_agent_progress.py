#!/usr/bin/env python3
"""Watch a Claude Code JSONL session log and print progress updates.

Polls for the latest JSONL file and prints a one-line status summary
every --interval seconds. Designed to run as a background process.

Usage: python3 watch_agent_progress.py --newer-than /tmp/final-task.md [--interval 30]
"""
import argparse
import glob
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional


LOG_PATTERNS = {
    "minimax": "~/.claude/projects/**/*.jsonl",
    "codex": "~/.codex/sessions/**/*.jsonl",
}

CODEX_LOOKBACK_LINES = 50
CODEX_NOISE_TYPES = {
    "?",
    "event_msg",
    "response_item",
    "item.started",
    "item.completed",
    "session_meta",
    "token_count",
    "task_started",
    "task_complete",
    "user_message",
    "reasoning",
    "function_call_output",
    "custom_tool_call_output",
}


def _set_meaningful_type(status: dict, type_name: str) -> None:
    if status["last_type"] in CODEX_NOISE_TYPES:
        status["last_type"] = type_name


def find_logfile(newer_than: Path, backend: str = "minimax") -> Optional[str]:
    """Find the most recent JSONL file newer than the reference file."""
    ref_mtime = newer_than.stat().st_mtime if newer_than.exists() else 0
    pattern = LOG_PATTERNS.get(backend, LOG_PATTERNS["minimax"])
    candidates = glob.glob(os.path.expanduser(pattern), recursive=True)
    for f in sorted(candidates, key=os.path.getmtime, reverse=True):
        if os.path.getmtime(f) > ref_mtime:
            return f
    return None


def _parse_claude_event(ev: dict, status: dict) -> bool:
    """Parse a Claude Code JSONL event. Returns True if status was updated."""
    status["last_type"] = ev.get("type", "?")
    if ev.get("type") != "assistant":
        return False
    for block in reversed(ev.get("message", {}).get("content", [])):
        if block.get("type") == "tool_use" and not status["last_tool"]:
            status["last_tool"] = block.get("name", "?")
        elif block.get("type") == "text" and not status["last_text"]:
            text = block.get("text", "").strip()
            if text:
                status["last_text"] = text[:200]
    return bool(status["last_tool"] or status["last_text"])


def _parse_codex_event(ev: dict, status: dict) -> bool:
    """Parse a Codex rollout JSONL event. Returns True if status was updated."""
    event_type = ev.get("type", "?")
    if status["last_type"] == "?":
        status["last_type"] = event_type

    payload = ev.get("payload")
    if isinstance(payload, dict):
        payload_type = payload.get("type", event_type)
        if status["last_type"] in CODEX_NOISE_TYPES:
            status["last_type"] = payload_type

        if event_type == "event_msg":
            if payload_type == "agent_message":
                text = payload.get("message", "").strip()
                if text:
                    if not status["last_text"]:
                        status["last_text"] = text[:200]
                    _set_meaningful_type(status, "agent_message")
                    return True
            if payload_type in ("token_count", "task_started", "task_complete", "user_message"):
                return False

        if event_type == "response_item":
            if payload_type in (
                "reasoning",
                "function_call_output",
                "custom_tool_call_output",
            ):
                return False

            if payload_type == "message" and payload.get("role") == "assistant":
                for block in reversed(payload.get("content", [])):
                    if not isinstance(block, dict):
                        continue
                    if block.get("type") in ("output_text", "text"):
                        text = block.get("text", "").strip()
                        if text:
                            if not status["last_text"]:
                                status["last_text"] = text[:200]
                            _set_meaningful_type(status, payload_type)
                            return True
                return False

            if payload_type in ("function_call", "custom_tool_call", "web_search_call"):
                if not status["last_tool"]:
                    status["last_tool"] = payload.get("name", payload_type)
                _set_meaningful_type(status, payload_type)
                return True

    item = ev.get("item")
    if isinstance(item, dict):
        item_type = item.get("type", event_type)
        if item_type == "agent_message":
            text = item.get("text", "").strip()
            if text:
                if not status["last_text"]:
                    status["last_text"] = text[:200]
                _set_meaningful_type(status, item_type)
                return True
        if item_type == "file_change":
            changes = item.get("changes", [])
            if changes:
                path = changes[0].get("path", "")
                if not status["last_tool"]:
                    status["last_tool"] = f"file_change:{Path(path).name}" if path else "file_change"
            else:
                if not status["last_tool"]:
                    status["last_tool"] = "file_change"
            _set_meaningful_type(status, item_type)
            return True
        if item_type == "todo_list":
            return False

    # Older Codex event shapes use a payload containing content blocks.
    payload = ev.get("payload", ev)
    msg_type = payload.get("type", event_type)
    if status["last_type"] in CODEX_NOISE_TYPES:
        status["last_type"] = msg_type

    # Assistant messages
    if msg_type in ("assistant", "response.output_item.done"):
        content = payload.get("content", payload.get("item", {}).get("content", []))
        if isinstance(content, str):
            if not status["last_text"]:
                status["last_text"] = content.strip()[:200]
            _set_meaningful_type(status, msg_type)
            return True
        if isinstance(content, list):
            for block in reversed(content):
                if isinstance(block, str):
                    if not status["last_text"]:
                        status["last_text"] = block.strip()[:200]
                    _set_meaningful_type(status, msg_type)
                    return True
                btype = block.get("type", "")
                if btype in ("function_call", "tool_use") and not status["last_tool"]:
                    status["last_tool"] = block.get("name", block.get("function", {}).get("name", "?"))
                    _set_meaningful_type(status, btype)
                elif btype in ("text", "output_text") and not status["last_text"]:
                    text = block.get("text", "").strip()
                    if text:
                        status["last_text"] = text[:200]
                        _set_meaningful_type(status, btype)
            return bool(status["last_tool"] or status["last_text"])
    return False


def get_status(logfile: str, backend: str = "minimax") -> dict:
    """Read the last few events and extract status info."""
    status = {
        "events": 0,
        "last_type": "?",
        "last_tool": None,
        "last_text": None,
    }

    try:
        with open(logfile) as f:
            lines = f.readlines()
    except OSError:
        return status

    status["events"] = len(lines)
    parser = _parse_codex_event if backend == "codex" else _parse_claude_event

    # Scan recent lines for the most recent useful content.
    lookback = CODEX_LOOKBACK_LINES if backend == "codex" else 10
    for line in reversed(lines[-lookback:]):
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue

        parser(ev, status)
        if backend == "codex":
            if status["last_text"] and status["last_tool"]:
                break
        elif status["last_text"] or status["last_tool"]:
            break

    return status


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--newer-than", type=Path, required=True,
        help="Reference file — only consider JSONL files newer than this",
    )
    parser.add_argument(
        "--interval", type=int, default=30,
        help="Seconds between progress updates (default: 30)",
    )
    parser.add_argument(
        "--backend", choices=["minimax", "codex"], default="minimax",
        help="Agent backend (determines log location and format)",
    )
    args = parser.parse_args()

    time.sleep(10)  # initial delay for session to start

    while True:
        now = datetime.now().strftime("%H:%M:%S")
        logfile = find_logfile(args.newer_than, args.backend)

        if logfile:
            s = get_status(logfile, args.backend)
            tool = s["last_tool"] or "n/a"
            text = s["last_text"] or "(none)"
            print(
                f"[progress] {now} | {s['events']} events | "
                f"type: {s['last_type']} | tool: {tool} | text: {text}",
                flush=True,
            )
        else:
            print(f"[progress] {now} | waiting for session to start...", flush=True)

        time.sleep(args.interval)


if __name__ == "__main__":
    main()
