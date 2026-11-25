"""``Operator.report`` that also lands in the debug log.

Blender resolves RNA functions such as ``report`` before Python class attributes, so an
override on the operator class is never called; the operators call this function instead."""
from __future__ import annotations

from .. import _debuglog


def report(operator, level: str, message: str) -> None:
    operator.report({level}, message)
    _debuglog.log_report(level, operator.bl_idname, message)
