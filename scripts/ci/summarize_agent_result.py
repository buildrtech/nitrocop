#!/usr/bin/env python3
"""Normalize Codex JSONL output into the workflow's agent-result.json shape."""

import json
import sys
from pathlib import Path


def _content_blocks(ev: dict) -> list[dict]:
    payload = ev.get("payload", ev)
    msg_type = payload.get("type", ev.get("type"))

    if msg_type == "assistant":
        content = payload.get("content", [])
    elif msg_type == "response.output_item.done":
        item = payload.get("item", {})
        content = item.get("content", payload.get("content", []))
    else:
        return []

    if isinstance(content, str):
        text = content.strip()
        return [{"type": "text", "text": text}] if text else []
    if isinstance(content, list):
        return [block for block in content if isinstance(block, dict)]
    return []


def _block_text(block: dict) -> str:
    btype = block.get("type")
    if btype in ("text", "output_text"):
        return block.get("text", "").strip()
    if btype == "output_text.delta":
        return block.get("delta", "").strip()
    return ""


def _last_text(events: list[dict]) -> str:
    for ev in reversed(events):
        blocks = _content_blocks(ev)
        for block in reversed(blocks):
            text = _block_text(block)
            if text:
                return text
    return ""


def _count_turns(events: list[dict]) -> int:
    turns = 0
    for ev in events:
        if _content_blocks(ev):
            turns += 1
    return turns


def _extract_cost(events: list[dict]):
    for ev in reversed(events):
        payload = ev.get("payload", ev)
        for obj in (payload, payload.get("response", {}), payload.get("item", {})):
            if isinstance(obj, dict) and obj.get("total_cost_usd") is not None:
                return obj["total_cost_usd"]
    return None


def main() -> int:
    if len(sys.argv) != 3:
        print(
            f"Usage: {sys.argv[0]} <events.jsonl> <last-message.txt>",
            file=sys.stderr,
        )
        return 1

    events_path = Path(sys.argv[1])
    last_message_path = Path(sys.argv[2])

    events: list[dict] = []
    if events_path.exists():
        with events_path.open() as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

    result_text = ""
    if last_message_path.exists():
        result_text = last_message_path.read_text().strip()
    if not result_text:
        result_text = _last_text(events)

    summary = {
        "backend": "codex",
        "events": len(events),
        "num_turns": _count_turns(events),
        "total_cost_usd": _extract_cost(events),
        "result": result_text,
    }
    json.dump(summary, sys.stdout)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
