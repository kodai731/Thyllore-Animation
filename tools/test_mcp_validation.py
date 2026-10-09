"""Tests for find_last_validation_summary from tools/mcp/thyllore_mcp.py."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from tools.mcp.thyllore_mcp import find_last_validation_summary


def test_empty_string():
    assert find_last_validation_summary("") is None


def test_no_match():
    assert find_last_validation_summary("some random log line\nanother line\n") is None


def test_single_line():
    text = "validation errors: 0, warnings: 1"
    assert find_last_validation_summary(text) == "validation errors: 0, warnings: 1"


def test_multiple_lines_first_match():
    text = "some log\nvalidation errors: 2, warnings: 3, first at frame 5: ...\nmore log\n"
    assert find_last_validation_summary(text) == "validation errors: 2, warnings: 3, first at frame 5: ..."


def test_multiple_matches_returns_last():
    text = (
        "some log\n"
        "validation errors: 1, warnings: 0\n"
        "middle line\n"
        "validation errors: 2, warnings: 1, first at frame 10: ...\n"
        "end\n"
    )
    assert find_last_validation_summary(text) == "validation errors: 2, warnings: 1, first at frame 10: ..."


def test_whitespace_stripped():
    text = "   validation errors: 0, warnings: 0   \n"
    assert find_last_validation_summary(text) == "validation errors: 0, warnings: 0"
