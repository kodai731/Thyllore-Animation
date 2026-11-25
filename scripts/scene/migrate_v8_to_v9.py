"""Rewrite version 8 scene files to version 9: the model becomes a scene entity.

    python3 scripts/scene/migrate_v8_to_v9.py [--backup DIR] assets/scenes/*.scene.ron

The top-level `model: (path: ..)` block is removed. A model file becomes the last entry of
`entities`, an entity named after the file with a `model_source` component holding the same path;
an empty path or a generated mesh names no file and leaves no entity. Everything else is copied
byte for byte.
"""
from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path, PurePosixPath, PureWindowsPath

GENERATED_MESH = "Generated Mesh"
UNIT = "    "

MODEL_BLOCK = re.compile(r'^(?P<indent>[ \t]*)model: \(\s*path: "(?P<path>[^"]*)",?\s*\),?[ \t]*\n', re.M)
EMPTY_ENTITIES = re.compile(r"^(?P<indent>[ \t]*)entities: \[\s*\],?[ \t]*\n", re.M)
ENTITIES_OPEN = re.compile(r"^(?P<indent>[ \t]*)entities: \[[ \t]*\n", re.M)


def names_model_file(model_path: str) -> bool:
    return model_path != "" and model_path != GENERATED_MESH


def model_entity_name(model_path: str) -> str:
    path_flavour = PureWindowsPath if "\\" in model_path else PurePosixPath
    return path_flavour(model_path).stem or "model"


def render_model_entity(model_path: str, indent: str) -> str:
    lines = [
        "(",
        f'{UNIT}name: "{model_entity_name(model_path)}",',
        f"{UNIT}components: {{",
        f'{UNIT * 2}"model_source": (',
        f'{UNIT * 3}path: "{model_path}",',
        f"{UNIT * 2}),",
        f"{UNIT}}},",
        "),",
    ]
    return "".join(f"{indent}{line}\n" for line in lines)


def find_list_close(text: str, list_open_end: int) -> int:
    """Index of the `]` that closes the list whose `[` ends just before `list_open_end`."""
    depth = 1
    in_string = False
    position = list_open_end
    while depth > 0:
        char = text[position]
        if in_string:
            if char == "\\":
                position += 1
            elif char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        position += 1
    return position - 1


def append_model_entity(text: str, model_path: str) -> str:
    empty_list = EMPTY_ENTITIES.search(text)
    if empty_list:
        indent = empty_list.group("indent")
        entity = render_model_entity(model_path, indent + UNIT)
        return f"{text[:empty_list.start()]}{indent}entities: [\n{entity}{indent}],\n{text[empty_list.end():]}"

    list_open = ENTITIES_OPEN.search(text)
    if list_open:
        indent = list_open.group("indent")
        list_close = find_list_close(text, list_open.end())
        close_line_start = text.rindex("\n", 0, list_close) + 1
        entity = render_model_entity(model_path, indent + UNIT)
        return f"{text[:close_line_start]}{entity}{text[close_line_start:]}"

    scene_close = text.rindex(")")
    entity = render_model_entity(model_path, UNIT * 2)
    return f"{text[:scene_close]}{UNIT}entities: [\n{entity}{UNIT}],\n{text[scene_close:]}"


def migrate_text(text: str) -> str:
    model_block = MODEL_BLOCK.search(text)
    if model_block is None:
        raise ValueError("no top-level model block")

    model_path = model_block.group("path")
    migrated = text[:model_block.start()] + text[model_block.end():]
    if names_model_file(model_path):
        migrated = append_model_entity(migrated, model_path)
    return migrated.replace("version: 8,", "version: 9,", 1)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--backup", type=Path, help="copy each original here before rewriting")
    parser.add_argument("scenes", nargs="+", type=Path)
    args = parser.parse_args()

    for scene in args.scenes:
        text = scene.read_text()
        if "version: 8," not in text:
            print(f"skip {scene}: not version 8")
            continue
        migrated = migrate_text(text)
        if args.backup:
            args.backup.mkdir(parents=True, exist_ok=True)
            shutil.copy2(scene, args.backup / scene.name)
        scene.write_text(migrated)
        print(f"migrated {scene}")


if __name__ == "__main__":
    main()
