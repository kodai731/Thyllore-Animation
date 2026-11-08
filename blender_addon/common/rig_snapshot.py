"""One read of a rig from bpy: the bones with their frames, the bound meshes and the skin surfaces, as the wheel's inputs."""
from __future__ import annotations

from dataclasses import dataclass, field
from functools import cached_property

import bpy

try:
    import thyllore_avatar_api as avatar_api  # type: ignore
except ImportError:
    avatar_api = None  # type: ignore

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


def rest_matrices(arm) -> list[list[list[float]]]:
    return [matrix_rows(bone.matrix_local) for bone in arm.data.bones]


def bone_specs(arm, armature_world, rest_rows, scale_length: float) -> list[dict]:
    """The wheel's bone specs (name, parent, rest position, bind matrix) of ``arm.data.bones``
    from matrices already read."""
    bones = arm.data.bones
    return avatar_api.bone_specs({
        "names": [bone.name for bone in bones],
        "parents": [None if bone.parent is None else bones.find(bone.parent.name) for bone in bones],
        "armature_world": armature_world,
        "rest_matrices": rest_rows,
        "scale_length": scale_length,
    })


def read_bones(context, arm) -> list[dict]:
    """The bone specs alone, for callers that never touch the bound meshes (overlays, previews)."""
    scale_length = context.scene.unit_settings.scale_length
    return bone_specs(arm, matrix_rows(arm.matrix_world), rest_matrices(arm), scale_length)


@dataclass
class RigSnapshot:
    """What every wheel call reads from one rig: ``bones`` carry ``bind_matrix`` (rest frame in metres),
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
    rest_rows = rest_matrices(arm)

    return RigSnapshot(
        arm=arm,
        scale_length=scale_length,
        armature_world=armature_world,
        rest_matrices=rest_rows,
        bones=bone_specs(arm, armature_world, rest_rows, scale_length),
        meshes=bound_meshes(arm),
    )
