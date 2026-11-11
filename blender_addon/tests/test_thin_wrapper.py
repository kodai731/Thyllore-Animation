"""Computation lives in the crates; blender_addon/ only reads and writes bpy.

No Python numeric library (numpy, math) may be imported under blender_addon/, and no
.inverted() matrix inversion may appear there: composing matrices belongs in the wheel.
Tests, developer tools and the vendored wheels are outside the rule.
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ADDON_DIR = REPO_ROOT / "blender_addon"
COMMON_DIR = ADDON_DIR / "common"
EXEMPT_FILES = {"coordinates.py", "effect_properties.py"}
SKIPPED_PARTS = {"__pycache__", "tests", "tools", "wheels-extracted"}


def _python_files():
    """Yield every addon *.py: not the tests, the developer tools or the vendored wheels."""
    for path in ADDON_DIR.rglob("*.py"):
        if SKIPPED_PARTS.isdisjoint(path.parts):
            yield path


def test_addon_does_not_import_numeric_libraries() -> None:
    """Every addon *.py must not import numpy or math.

    Exempt exactly coordinates.py and effect_properties.py.
    """
    patterns = [
        re.compile(r"^import numpy\b"),
        re.compile(r"^import numpy as\b"),
        re.compile(r"^from numpy\b"),
        re.compile(r"^import math\b"),
        re.compile(r"^from math import\b"),
    ]
    offenders = []
    for path in _python_files():
        if path.name in EXEMPT_FILES:
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            for pattern in patterns:
                if pattern.match(stripped):
                    offenders.append(
                        f"{path.relative_to(REPO_ROOT)}:{line_number}: {stripped}"
                    )
    assert not offenders, "numeric library import found:\n" + "\n".join(offenders)


RIG_READERS = ("avatar_api.bone_specs(", "matrix_local", "skin_input(")
RIG_READER_HOME = COMMON_DIR / "rig_snapshot.py"
RIG_READER_DEFINITIONS = {"skin_input.py"}


def test_rig_is_read_only_by_the_snapshot() -> None:
    """Bone specs, rest frames, bind matrices and skin surfaces are built in one place; operators take a RigSnapshot."""
    offenders = []
    for path in _python_files():
        if path == RIG_READER_HOME or path.name in RIG_READER_DEFINITIONS:
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if any(reader in line for reader in RIG_READERS):
                offenders.append(f"{path.relative_to(REPO_ROOT)}:{line_number}: {line.strip()}")
    assert not offenders, "rig read outside rig_snapshot.py:\n" + "\n".join(offenders)


def test_no_matrix_inversions() -> None:
    """mathutils may build Matrix/Vector and multiply them; composing with .inverted() belongs in the wheel."""
    offenders = []
    for path in _python_files():
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if ".inverted()" in line:
                offenders.append(f"{path.relative_to(REPO_ROOT)}:{line_number}: {line.strip()}")
    assert not offenders, "matrix inversion found:\n" + "\n".join(offenders)


def test_root_attachment_names_are_spelled_in_the_wheel_only(avatar_api) -> None:
    """The sidecar attachment values come from root_attachments(); Python derives its enum identifiers from them."""
    names = [entry["name"] for entry in avatar_api.root_attachments()]
    assert names
    quoted = [re.compile(rf"""["']{re.escape(name)}["']""") for name in names]
    offenders = []
    for path in _python_files():
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if any(pattern.search(line) for pattern in quoted):
                offenders.append(f"{path.relative_to(REPO_ROOT)}:{line_number}: {line.strip()}")
    assert not offenders, "root attachment literal found:\n" + "\n".join(offenders)
