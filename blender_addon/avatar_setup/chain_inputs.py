"""What the chain generation reads from Blender: the submesh, its geometry, the rig's bones and colliders,
and the chains the user assigned, all as the wheel's inputs (engine metres)."""
from __future__ import annotations

import bpy

try:
    import thyllore_avatar_api as avatar_api  # type: ignore
except ImportError:
    avatar_api = None  # type: ignore

from ..common import coordinates
from ..common.chain_bones import VertexGroupNames, given_chains_from_groups, group_vertices, joints_from_bones, vertex_tex_coords
from ..common.skin_input import collider_inputs
from ..common.spring_properties import humanoid_overrides_to_dict, spring_settings_to_edit
from ..common.submesh_mask import material_triangles
from ..common.rig_snapshot import bound_meshes

VERTEX_GROUP_NAMES = VertexGroupNames(avatar_api.vertex_group_names()) if avatar_api is not None else None


def _chain_roles() -> dict[str, str]:
    """{role: default_root} from the wheel's chain_roles()."""
    return {entry["role"]: entry["default_root"] for entry in avatar_api.chain_roles()}


def generatable_roles() -> set[str]:
    """The submesh roles that can grow chain bones."""
    return set(_chain_roles().keys())


def generatable_submeshes(arm) -> list[tuple[object, object]]:
    """(mesh object, role item) pairs whose submesh role can grow chain bones."""
    roles = generatable_roles()
    return [
        (mesh_obj, item)
        for mesh_obj in bound_meshes(arm)
        for item in mesh_obj.thyllore_submesh.roles
        if item.role in roles
    ]


def _material_slot_index(obj, material_name: str) -> int | None:
    for index, slot in enumerate(obj.material_slots):
        if slot.material is not None and slot.material.name == material_name:
            return index
    return None


def resolve_submesh(arm, mesh_name: str, material: str):
    """The mesh object, its submesh row (role and attachment) and its material slot for a hair or garment
    submesh of the rig."""
    obj = bpy.data.objects.get(mesh_name)
    if obj is None or obj.type != "MESH" or obj not in bound_meshes(arm):
        raise ValueError(f"{mesh_name!r} is not a mesh bound to {arm.name!r}")
    role_item = next((item for item in obj.thyllore_submesh.roles if item.material == material), None)
    if role_item is None or role_item.role not in generatable_roles():
        raise ValueError(f"{material!r} on {obj.name!r} is not a hair or garment submesh")
    material_index = _material_slot_index(obj, material)
    if material_index is None:
        raise ValueError(f"{obj.name!r} has no material slot {material!r}")
    return obj, role_item, material_index


def engine_point(obj, local_point, scale_length: float) -> list[float]:
    return list(coordinates.blender_to_engine_metres(tuple(obj.matrix_world @ local_point), scale_length))


def root_engine_position(arm, root_bone: str, scale_length: float) -> list[float]:
    return engine_point(arm, arm.data.bones[root_bone].head_local, scale_length)


def submesh_triangles(mesh, material_index: int) -> list:
    """The loop triangles of one material slot, as vertex index triples."""
    mesh.calc_loop_triangles()
    triangle_vertices = []
    triangle_materials = []
    for lt in mesh.loop_triangles:
        triangle_vertices.extend(lt.vertices)
        triangle_materials.append(lt.material_index)
    return material_triangles(triangle_vertices, triangle_materials, material_index)[0]


def submesh_tex_coords(mesh) -> list[tuple[float, float]]:
    """Per-vertex UV of the active layer; none without a UV layer."""
    uv_layer = mesh.uv_layers.active
    if uv_layer is None:
        return []
    loop_vertices = [l.vertex_index for l in mesh.loops]
    loop_uvs = [tuple(d.uv) for d in uv_layer.data]
    return vertex_tex_coords(loop_vertices, loop_uvs, len(mesh.vertices))


def humanoid_bone_indices(arm, settings, bones) -> dict[str, int]:
    """Unity role name to armature bone index, the inferred mapping with the user's overrides on top."""
    roles = dict(avatar_api.infer_humanoid_mapping({"bones": bones}).get("mapping", {}))
    roles.update(humanoid_overrides_to_dict(settings))
    indices = {}
    for role, bone_name in roles.items():
        index = arm.data.bones.find(bone_name)
        if index >= 0:
            indices[role] = index
    return indices


def body_collider_inputs(arm, settings, bones) -> list[dict]:
    model_spring = avatar_api.spring_colliders_to_model({"spring": spring_settings_to_edit(settings), "bones": bones})
    bone_index = {bone.name: index for index, bone in enumerate(arm.data.bones)}
    return collider_inputs(model_spring["colliders"], bone_index)


def vertex_group_weights(obj):
    return [(v.index, [(g.group, g.weight) for g in v.groups]) for v in obj.data.vertices]


def marked_covered_vertices(obj) -> list[int]:
    """The vertices of the covered group, or none when the group does not exist (the coverage is then measured)."""
    group = obj.vertex_groups.get(VERTEX_GROUP_NAMES.covered)
    if group is None:
        return []
    return group_vertices(vertex_group_weights(obj), group.index)


def _chain_bones(arm, registered: dict[str, list[str]], chain_name: str) -> list:
    """The armature bones of a registered chain, root to tip; none when any of them is missing."""
    bones = [arm.data.bones.get(name) for name in registered.get(chain_name, [])]
    if not bones or any(bone is None for bone in bones):
        return []
    return bones


def given_chains(obj, arm, registered: dict[str, list[str]], scale_length: float) -> list[dict]:
    """The chains whose vertices the user assigned in membership groups, with the joints of the chains whose
    bones exist on the armature; none when the split is computed."""
    groups = [(vg.name, vg.index) for vg in obj.vertex_groups]
    given = given_chains_from_groups(vertex_group_weights(obj), groups, VERTEX_GROUP_NAMES)
    for chain in given:
        bones = _chain_bones(arm, registered, chain["name"])
        if bones:
            chain["joints"] = joints_from_bones(
                [(engine_point(arm, b.head_local, scale_length), engine_point(arm, b.tail_local, scale_length)) for b in bones]
            )
    return given


def read_back_root(arm, registered: dict[str, list[str]], given: list[dict]) -> str | None:
    """The bone the read-back chains hang from, so a regenerate keeps the root the first generate chose."""
    for chain in given:
        if "joints" not in chain:
            continue
        parent = arm.data.bones[registered[chain["name"]][0]].parent
        if parent is not None:
            return parent.name
    return None
