"""What the chain generation writes into Blender: the chain bones, the vertex groups and the joint radii."""
from __future__ import annotations

from mathutils import Vector

import bpy

try:
    import thyllore_avatar_api as avatar_api  # type: ignore
except ImportError:
    avatar_api = None  # type: ignore

from ..common.chain_bones import COVERED_GROUP, chain_of_membership_group, membership_group_name
from .operators import matrix_rows


def _armature_local_pairs(arm, specs) -> list[tuple[Vector, Vector]]:
    """(head, tail) of every spec in the armature's space, where edit bones are placed."""
    points = avatar_api.armature_local_points({
        "armature_world": matrix_rows(arm.matrix_world),
        "points": [point for _, _, head, tail in specs for point in (head, tail)],
    })
    return [(Vector(points[i]), Vector(points[i + 1])) for i in range(0, len(points), 2)]


def add_bones(context, arm, specs):
    """Add bones in edit mode from (name, parent, head, tail) specs in Blender world space."""
    for name, *_ in specs:
        if name in arm.data.bones:
            raise ValueError(f"Bone {name!r} already exists; reset the chain generation to replace it")
    placements = _armature_local_pairs(arm, specs)
    context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    try:
        edit_bones = arm.data.edit_bones
        for (name, parent_name, _, _), (head, tail) in zip(specs, placements):
            bone = edit_bones.new(name)
            bone.head = head
            bone.tail = tail
            bone.parent = edit_bones[parent_name]
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")


def remove_bones(context, arm, bone_names):
    """Remove bones by name in edit mode; descendants of a removed bone are removed by Blender."""
    if not bone_names:
        return
    context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    try:
        edit_bones = arm.data.edit_bones
        for name in bone_names:
            bone = edit_bones.get(name)
            if bone is not None:
                edit_bones.remove(bone)
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")


def remove_vertex_groups(obj, bone_names):
    for name in bone_names:
        group = obj.vertex_groups.get(name)
        if group is not None:
            obj.vertex_groups.remove(group)


def write_weights(obj, groups, vertices: set[int], bone_names: set[str]):
    """Write the bone weights of ``vertices``: clear them from every bone group, then REPLACE; the other vertices keep
    their weights and the non-bone groups (modifier masks, membership, coverage) are untouched."""
    for vg in obj.vertex_groups:
        if vg.name in bone_names:
            vg.remove(list(vertices))
    for bone_name, influences in groups.items():
        vg = obj.vertex_groups.get(bone_name)
        if vg is None:
            vg = obj.vertex_groups.new(name=bone_name)
        for v_idx, weight in influences:
            if v_idx in vertices:
                vg.add([v_idx], weight, "REPLACE")


def _replace_group(obj, name: str, vertices: list[int]) -> None:
    group = obj.vertex_groups.get(name)
    if group is None:
        group = obj.vertex_groups.new(name=name)
    group.remove([v.index for v in obj.data.vertices])
    if vertices:
        group.add([int(v) for v in vertices], 1.0, "REPLACE")


def write_membership_groups(obj, chains: list[dict], given: list[dict]) -> None:
    """Record the computed membership so the user can reassign it; assigned groups are the input and stay as they are."""
    given_names = {chain["name"] for chain in given}
    for chain in chains:
        if chain["name"] not in given_names:
            _replace_group(obj, membership_group_name(chain["name"]), chain["vertices"])


def write_covered_group(obj, covered: list[int], marked: list[int]) -> None:
    """Record the measured coverage so the user can repaint it; a painted group is the input and stays as it is."""
    if not marked:
        _replace_group(obj, COVERED_GROUP, covered)


def update_hit_radii(settings, chains: list[dict], read_back: set[str]) -> int:
    """Give the joints of the read-back chains the hit radius this generation measured; the count changed."""
    radii = {
        (chain["name"], joint["name"]): float(joint["hit_radius"])
        for chain in chains
        if chain["name"] in read_back
        for joint in chain["joints"]
    }
    changed = 0
    for chain in settings.chains:
        for joint in chain.joints:
            measured = radii.get((chain.name, joint.bone))
            if measured is not None and joint.hit_radius != measured:
                joint.hit_radius = measured
                changed += 1
    return changed
