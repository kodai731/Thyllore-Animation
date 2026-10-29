"""Tests for blender_addon.common.logged_report — no bpy."""

from __future__ import annotations

from blender_addon import _debuglog
from blender_addon.common.logged_report import report


class _FakeOperator:
    bl_idname = "thyllore.fake"

    def __init__(self):
        self.reported = []

    def report(self, type, message):
        self.reported.append((type, message))


def test_report_reaches_both_the_ui_and_the_debug_log(monkeypatch):
    logged = []
    monkeypatch.setattr(_debuglog, "log_report", lambda level, source, message: logged.append((level, source, message)))
    op = _FakeOperator()

    report(op, "ERROR", "no bone runs")

    assert op.reported == [({"ERROR"}, "no bone runs")]
    assert logged == [("ERROR", "thyllore.fake", "no bone runs")]
