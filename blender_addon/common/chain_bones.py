"""Pure Python helpers for chain bone generation output.

No bpy imports: these functions work on the data returned by the wheel's
generate_chains call (engine coordinates, Y-up metres) and are consumed by the
addon's bpy operator, which converts to Blender space with ``coordinates``.
"""
from __future__ import annotations

from .coordinates import engine_to_blender_point


def submesh_triangles(
    triangle_vertices: list[int],
    triangle_materials: list[int],
    material_index: int,
) -> list[list[int]]:
    """Extract triangles belonging to one material from Blender loop data.

    ``triangle_vertices`` is the flat list of vertex indices from
    ``mesh.loop_triangles`` (3 per triangle).  ``triangle_materials`` is the
    parallel list of ``material_index`` values.  Returns only the triangles
    whose material matches ``material_index``, each as a 3-element list.
    """
    result: list[list[int]] = []
    for i in range(0, len(triangle_vertices), 3):
        if triangle_materials[i // 3] == material_index:
            result.append(
                [triangle_vertices[i], triangle_vertices[i + 1], triangle_vertices[i + 2]]
            )
    return result


def output_to_blender(output: dict) -> dict:
    """The same output with every joint head and chain tail converted to Blender space."""
    chains = [
        {
            **chain,
            "joints": [
                {**joint, "head": engine_to_blender_point(joint["head"])} for joint in chain["joints"]
            ],
            "tail": engine_to_blender_point(chain["tail"]),
        }
        for chain in output["chains"]
    ]
    return {**output, "chains": chains}


def settings_from_properties(ui_params: list[dict], owner) -> dict[str, float]:
    """Read every setting of the table from the attributes of ``owner`` (an operator or property group)."""
    return {param["name"]: float(getattr(owner, param["name"])) for param in ui_params}


def bone_specs(
    output: dict, root_bone: str
) -> list[tuple[str, str | None, tuple[float, float, float], tuple[float, float, float]]]:
    """Build (name, parent, head, tail) specs for every joint in every chain.

    Within a chain each joint's tail is the next joint's head; the last joint's
    tail is the chain's ``tail``.  The first joint's parent is ``root_bone``,
    subsequent joints' parent is the preceding joint's name.
    """
    specs: list[tuple[str, str | None, tuple[float, float, float], tuple[float, float, float]]] = []
    for chain in output["chains"]:
        joints = chain["joints"]
        prev_name: str | None = root_bone
        for i, joint in enumerate(joints):
            name = joint["name"]
            head = tuple(joint["head"])
            if i + 1 < len(joints):
                tail = tuple(joints[i + 1]["head"])
            else:
                tail = tuple(chain["tail"])
            specs.append((name, prev_name, head, tail))
            prev_name = name
    return specs


def group_weights(
    output: dict, root_bone: str
) -> dict[str, list[tuple[int, float]]]:
    """Group vertex influences by bone name.

    Returns ``{bone_name: [(vertex_index, weight), ...]}``.  The special target
    ``"root"`` is renamed to ``root_bone``.
    """
    groups: dict[str, list[tuple[int, float]]] = {}
    for entry in output["weights"]:
        vertex = int(entry["vertex"])
        for target, weight in entry["influences"]:
            bone_name = root_bone if target == "root" else target
            groups.setdefault(bone_name, []).append((vertex, weight))
    return groups
