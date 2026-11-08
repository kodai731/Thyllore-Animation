"""Rename the chains an older addon generated under ``<role>_<label>_<column>`` to the names the wheel gives
them now, so a regenerate reads their assignment and bones back instead of replacing them."""
from __future__ import annotations

try:
    import thyllore_ml_core as tml  # type: ignore
except ImportError:
    tml = None  # type: ignore

from ..common.chain_bones import membership_group_name
from ..common.spring_properties import generated_chain_bones


def rename_legacy_chains(arm, obj, settings, origin: dict, target: str) -> int:
    """Rename the legacy chains generated on ``origin`` with their bones, vertex groups and bone collections;
    returns the number of chains renamed."""
    registered = generated_chain_bones(settings, origin)
    renames = dict(tml.legacy_chain_renames(target, list(registered)))
    for chain in settings.chains:
        new_name = renames.get(chain.name)
        if new_name is None:
            continue
        bone_renames = {bone: new_name + bone[len(chain.name) :] for bone in registered[chain.name]}
        _rename_bones(arm, bone_renames)
        for joint in chain.joints:
            joint.bone = bone_renames.get(joint.bone, joint.bone)
        if chain.tail:
            chain.tail = bone_renames.get(chain.tail, chain.tail)
        _rename_vertex_group(obj, membership_group_name(chain.name), membership_group_name(new_name))
        _rename_bone_collection(arm.data, chain.name, new_name)
        chain.name = new_name
    return len(renames)


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
