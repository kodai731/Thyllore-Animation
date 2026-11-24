"""Rewrite the timeline record of version 9 scene files: a selected user clip is named by its file.

    python3 scripts/scene/migrate_v9_timeline_clip_file.py [--backup DIR] assets/scenes/*.scene.ron

`"current_clip": Some("<name>")` used to select the first clip file of the scene with that name,
and a model clip only when no file had it. When one of the scene's `animation_clips` files holds a
clip of that name, the line becomes `"current_clip": None` followed by
`"current_clip_file": Some("<path of that file>")`. A name no clip file holds is a model clip and
stays as it is. Everything else is copied byte for byte.
"""
from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path

from migrate_v8_to_v9 import find_list_close

CLIP_FILE_LIST_OPEN = re.compile(r"^[ \t]*animation_clips: \[", re.M)
CLIP_FILE_PATH = re.compile(r'path: "(?P<path>[^"]*)"')
CLIP_NAME = re.compile(r'^\s*name: "(?P<name>(?:[^"\\]|\\.)*)"', re.M)
SELECTED_CLIP = re.compile(r'^(?P<indent>[ \t]*)"current_clip": Some\("(?P<name>(?:[^"\\]|\\.)*)"\),[ \t]*\n', re.M)
SELECTED_CLIP_FILE = '"current_clip_file"'


def list_clip_files(scene_text: str) -> list[str]:
    list_open = CLIP_FILE_LIST_OPEN.search(scene_text)
    if list_open is None:
        return []
    list_close = find_list_close(scene_text, list_open.end())

    return CLIP_FILE_PATH.findall(scene_text[list_open.end():list_close])


def read_clip_name(clip_file: Path) -> str | None:
    if not clip_file.is_file():
        return None
    clip_name = CLIP_NAME.search(clip_file.read_text())
    return clip_name.group("name") if clip_name else None


def find_clip_file_named(scene_text: str, assets_dir: Path, clip_name: str) -> str | None:
    for clip_file in list_clip_files(scene_text):
        if read_clip_name(assets_dir / clip_file) == clip_name:
            return clip_file
    return None


def migrate_text(scene_text: str, assets_dir: Path) -> str | None:
    """The rewritten scene, or None when the scene already selects its clip the current way."""
    if SELECTED_CLIP_FILE in scene_text:
        return None
    selected_clip = SELECTED_CLIP.search(scene_text)
    if selected_clip is None:
        return None
    clip_file = find_clip_file_named(scene_text, assets_dir, selected_clip.group("name"))
    if clip_file is None:
        return None

    indent = selected_clip.group("indent")
    selection = f'{indent}"current_clip": None,\n{indent}{SELECTED_CLIP_FILE}: Some("{clip_file}"),\n'
    return scene_text[:selected_clip.start()] + selection + scene_text[selected_clip.end():]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--backup", type=Path, help="copy each original here before rewriting")
    parser.add_argument("scenes", nargs="+", type=Path)
    args = parser.parse_args()

    for scene in args.scenes:
        scene_text = scene.read_text()
        if "version: 9," not in scene_text:
            print(f"skip {scene}: not version 9")
            continue
        migrated = migrate_text(scene_text, scene.resolve().parent.parent)
        if migrated is None:
            print(f"keep {scene}: no user clip selected by name")
            continue
        if args.backup:
            args.backup.mkdir(parents=True, exist_ok=True)
            shutil.copy2(scene, args.backup / scene.name)
        scene.write_text(migrated)
        print(f"migrated {scene}")


if __name__ == "__main__":
    main()
