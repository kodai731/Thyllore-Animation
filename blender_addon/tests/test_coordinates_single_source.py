"""Only blender_addon/common/coordinates.py may spell out the Y-up / Z-up axis swap.

Every other Python file (addon, scripts, tools) converts through that module; a tuple that reorders
the components of one vector as x, z, y anywhere else is a second definition of the conversion.
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCANNED_DIRS = ("blender_addon", "scripts", "tools", "unity")
SINGLE_SOURCE = REPO_ROOT / "blender_addon" / "common" / "coordinates.py"

AXIS_SWAP = re.compile(
    r"\(\s*(?P<a>[\w\.\[\]]+)\s*,\s*-?\s*(?P<b>[\w\.\[\]]+)\s*,\s*-?\s*(?P<c>[\w\.\[\]]+)\s*\)"
)


def _is_axis_swap(match: re.Match) -> bool:
    a, b, c = match.group("a"), match.group("b"), match.group("c")
    indexed = [re.fullmatch(r"(\w+)\[(\d)\]", v) for v in (a, b, c)]
    if all(indexed):
        base = {m.group(1) for m in indexed}
        order = [int(m.group(2)) for m in indexed]
        return len(base) == 1 and order in ([0, 2, 1],)
    names = [v.lower() for v in (a, b, c)]
    return names in (["x", "z", "y"],)


def _python_files():
    for directory in SCANNED_DIRS:
        root = REPO_ROOT / directory
        if not root.exists():
            continue
        for path in root.rglob("*.py"):
            if ".venv" in path.parts or path in (SINGLE_SOURCE, Path(__file__).resolve()):
                continue
            yield path


def test_axis_swaps_live_only_in_coordinates_module() -> None:
    offenders = []
    for path in _python_files():
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in AXIS_SWAP.finditer(line):
                if _is_axis_swap(match):
                    offenders.append(f"{path.relative_to(REPO_ROOT)}:{line_number}: {line.strip()}")
    assert not offenders, "axis swap outside coordinates.py:\n" + "\n".join(offenders)
