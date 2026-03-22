#!/usr/bin/env python3
"""Tests for validate_codex_auth.py."""
import json
import os
import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "ci" / "validate_codex_auth.py"


def run(payload=None):
    env = os.environ.copy()
    if payload is None:
        env.pop("CODEX_AUTH_JSON", None)
    else:
        env["CODEX_AUTH_JSON"] = json.dumps(payload)
    return subprocess.run(
        [sys.executable, str(SCRIPT)],
        capture_output=True,
        text=True,
        env=env,
    )


def test_accepts_managed_auth():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "id_token": "eyJ-id",
            "account_id": "e7-account",
        },
        "last_refresh": "2026-03-20T03:31:56.783681Z",
    })
    assert result.returncode == 0
    assert "managed auth payload" in result.stdout
    assert "e7-acc" in result.stdout


def test_accepts_api_key_auth():
    result = run({
        "OPENAI_API_KEY": "sk-test",
        "tokens": None,
        "last_refresh": None,
    })
    assert result.returncode == 0
    assert "API key auth payload" in result.stdout


def test_rejects_missing_secret():
    result = run(None)
    assert result.returncode != 0
    assert "missing or empty" in result.stderr


def test_rejects_invalid_shape():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "",
            "refresh_token": "rt-refresh",
        },
    })
    assert result.returncode != 0
    assert "tokens.access_token is missing or empty" in result.stderr


def test_warns_on_missing_account_id():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
        },
        "last_refresh": "2026-03-20T03:31:56.783681Z",
    })
    assert result.returncode == 0
    assert "WARNING: tokens.account_id is missing or empty" in result.stderr


if __name__ == "__main__":
    test_accepts_managed_auth()
    test_accepts_api_key_auth()
    test_rejects_missing_secret()
    test_rejects_invalid_shape()
    test_warns_on_missing_account_id()
    print("All tests passed.")
