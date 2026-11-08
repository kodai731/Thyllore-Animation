"""Computation lives in the crates; blender_addon/ only reads and writes bpy.

No Python numeric library (numpy, math) may be imported in avatar_setup/ or common/,
and no .inverted() matrix inversion may appear in those directories: composing matrices
belongs in the wheel.
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
AVATAR_SETUP_DIR = REPO_ROOT / "blender_addon" / "avatar_setup"
COMMON_DIR = REPO_ROOT / "blender_addon" / "common"
EXEMPT_FILES = {"coordinates.py", "effect_properties.py"}


def _python_files():
    """Yield every *.py under avatar_setup/ and common/, skipping __pycache__."""
    for directory in (AVATAR_SETUP_DIR, COMMON_DIR):
        if not directory.exists():
            continue
        for path in directory.rglob("*.py"):
            if "__pycache__" in path.parts:
                continue
            yield path


def test_avatar_setup_does_not_import_numeric_libraries() -> None:
    """Every *.py under avatar_setup/ and common/ must not import numpy or math.

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


def test_no_matrix_inversions() -> None:
    """mathutils may build Matrix/Vector and multiply them; composing with .inverted() belongs in the wheel."""
    offenders = []
    for path in _python_files():
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if ".inverted()" in line:
                offenders.append(f"{path.relative_to(REPO_ROOT)}:{line_number}: {line.strip()}")
    assert not offenders, "matrix inversion found:\n" + "\n".join(offenders)
