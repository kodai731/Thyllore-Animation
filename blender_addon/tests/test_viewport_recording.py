"""Tests for the bpy-free helpers of blender_addon/common/viewport_recording.py."""

from __future__ import annotations

import importlib.util
import os
import sys
from pathlib import Path

_MODULE_PATH = os.path.join(os.path.dirname(__file__), "..", "common", "viewport_recording.py")
_spec = importlib.util.spec_from_file_location("viewport_recording", _MODULE_PATH)
viewport_recording = importlib.util.module_from_spec(_spec)  # type: ignore[arg-type]
sys.modules["viewport_recording"] = viewport_recording
_spec.loader.exec_module(viewport_recording)  # type: ignore[attr-defined]


def test_frame_numbers_include_end_and_honor_step():
    assert viewport_recording.frame_numbers(1, 10, 3) == [1, 4, 7, 10]
    assert viewport_recording.frame_numbers(5, 5, 1) == [5]


def test_frame_numbers_empty_for_inverted_range_or_bad_step():
    assert viewport_recording.frame_numbers(10, 1, 1) == []
    assert viewport_recording.frame_numbers(1, 10, 0) == []


def test_frames_directory_sits_next_to_video():
    directory = viewport_recording.frames_directory("/tmp/out/tornado.mp4")
    assert directory == Path("/tmp/out/tornado_frames")


def test_frame_paths_are_one_based_and_zero_padded():
    paths = viewport_recording.frame_paths("/tmp/out/tornado.mp4", 3)
    assert [p.name for p in paths] == ["frame_0001.png", "frame_0002.png", "frame_0003.png"]
    assert all(p.parent == Path("/tmp/out/tornado_frames") for p in paths)


def test_clear_frames_directory_removes_only_frame_files(tmp_path):
    (tmp_path / "frame_0001.png").write_bytes(b"x")
    (tmp_path / "keep.txt").write_bytes(b"x")
    viewport_recording.clear_frames_directory(tmp_path)
    assert sorted(p.name for p in tmp_path.iterdir()) == ["keep.txt"]
