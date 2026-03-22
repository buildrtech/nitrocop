#!/usr/bin/env python3
"""Compatibility CLI wrapper for corpus.check_cop."""

from corpus.check_cop import *  # noqa: F401,F403


if __name__ == "__main__":
    raise SystemExit(main())
