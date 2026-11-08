"""Rename the chains an older addon generated under ``<role>_<label>_<column>`` to the names the wheel gives
them now, so a regenerate or a reset reads their assignment and bones back instead of replacing them."""
from __future__ import annotations

try:
    import thyllore_avatar_api as avatar_api  # type: ignore
except ImportError:
    avatar_api = None  # type: ignore

from .chain_bones import chain_of_membership_group, membership_group_name
from .spring_properties import generated_chain_bones


def rename_legacy_chains(arm, obj, settings, origin: dict, target: str) -> int:
    """Rename the legacy chains of the submesh: the registered chains with their bones, every stored bone
    reference, their membership groups and bone collections; returns the number of chains renamed."""
    registered = generated_chain_bones(settings, origin)
    membership = _membership_groups(obj)
    candidates = sorted(set(registered) | set(membership))
    renames = dict(avatar_api.legacy_chain_renames(target, candidates))

    for old_name, new_name in renames.items():
        bone_renames = {bone: new_name + bone[len(old_name) :] for bone in registered.get(old_name, [])}
        _rename_bones(arm, bone_renames)
        rename_bone_references(settings, bone_renames)
        _rename_vertex_group(obj, membership_group_name(old_name), membership_group_name(new_name))
        _rename_bone_collection(arm.data, old_name, new_name)
        for chain in settings.chains:
            if chain.name == old_name:
                chain.name = new_name
    return len(renames)


def rename_bone_references(settings, bone_renames: dict[str, str]) -> None:
    """Point every stored bone name of the spring settings (joints, chain tails and centers, colliders) at
    the renamed bones; the properties are plain strings Blender does not update with the bone."""
    for chain in settings.chains:
        for joint in chain.joints:
            joint.bone = bone_renames.get(joint.bone, joint.bone)
        chain.tail = bone_renames.get(chain.tail, chain.tail)
        chain.center = bone_renames.get(chain.center, chain.center)
    for collider in settings.colliders:
        collider.bone = bone_renames.get(collider.bone, collider.bone)


def _membership_groups(obj) -> list[str]:
    chains = (chain_of_membership_group(vg.name) for vg in obj.vertex_groups)
    return [chain for chain in chains if chain is not None]


def _rename_bones(arm, bone_renames: dict[str, str]) -> None:
    """Rename armature bones; Blender renames the deform vertex groups of the bound meshes with them."""
    for old_name, new_name in bone_renames.items():
        bone = arm.data.bones.get(old_name)
        if bone is not None:
            bone.name = new_name


def _rename_vertex_group(obj, old_name: str, new_name: str) -> None:
    group = obj.vertex_groups.get(old_name)
    if group is not None:
        group.name = new_name


def _rename_bone_collection(armature_data, old_name: str, new_name: str) -> None:
    collection = armature_data.collections_all.get(old_name)
    if collection is not None:
        collection.name = new_name
