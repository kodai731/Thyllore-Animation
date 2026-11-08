"""One read of a rig from bpy: the bones with their frames, the bound meshes and the skin surfaces, as the wheel's inputs."""
from __future__ import annotations

from dataclasses import dataclass, field
from functools import cached_property

import bpy

from .avatar_sidecar import bone_specs
from .bone_frames import bone_bind_matrix, bone_rest_frame
from .skin_input import skin_input, skin_role_meshes


def matrix_rows(matrix) -> list[list[float]]:
    return [[matrix[i][j] for j in range(4)] for i in range(4)]


def bound_meshes(arm) -> list:
    """The mesh objects that reference ``arm`` through an ARMATURE modifier."""
    meshes = []
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        for mod in obj.modifiers:
            if mod.type == "ARMATURE" and mod.object is arm:
                meshes.append(obj)
                break
    return meshes


def _bone_table(arm):
    """Names, parent indices and world-space heads of ``arm.data.bones``."""
    bones = arm.data.bones
    names = []
    parent_indices = []
    world_heads = []
    for bone in bones:
        names.append(bone.name)
        parent_indices.append(None if bone.parent is None else bones.find(bone.parent.name))
        world_heads.append(tuple(arm.matrix_world @ bone.head_local))
    return names, parent_indices, world_heads


@dataclass
class RigSnapshot:
    """What every wheel call reads from one rig: ``bones`` carry ``rest_frame`` and ``bind_matrix``,
    ``rest_matrices`` are the bones' ``matrix_local`` rows, ``skins`` are read on first use."""

    arm: object
    scale_length: float
    armature_world: list[list[float]]
    rest_matrices: list[list[list[float]]]
    bones: list[dict]
    meshes: list = field(default_factory=list)

    @cached_property
    def skins(self) -> list[dict]:
        return [skin_input(mesh_obj, self.arm, self.scale_length) for mesh_obj in skin_role_meshes(self.meshes)]


def read_rig(context, arm) -> RigSnapshot:
    scale_length = context.scene.unit_settings.scale_length
    armature_world = matrix_rows(arm.matrix_world)
    rest_matrices = [matrix_rows(bone.matrix_local) for bone in arm.data.bones]

    names, parent_indices, world_heads = _bone_table(arm)
    rest_frames = [bone_rest_frame(armature_world, rows, scale_length) for rows in rest_matrices]
    bones = bone_specs(names, parent_indices, world_heads, scale_length, rest_frames)
    for spec, rows in zip(bones, rest_matrices):
        spec["bind_matrix"] = bone_bind_matrix(armature_world, rows, scale_length)

    return RigSnapshot(
        arm=arm,
        scale_length=scale_length,
        armature_world=armature_world,
        rest_matrices=rest_matrices,
        bones=bones,
        meshes=bound_meshes(arm),
    )
