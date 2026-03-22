#!/usr/bin/env python3
"""Compatibility CLI wrapper for corpus.verify_cop_locations."""

from corpus.verify_cop_locations import *  # noqa: F401,F403


if __name__ == "__main__":
    if "--test" in __import__("sys").argv:
        _run_tests()
    else:
        raise SystemExit(main())
