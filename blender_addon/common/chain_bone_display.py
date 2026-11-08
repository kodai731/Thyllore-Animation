"""Blender-side display of spring chain bones: one bone collection per chain, stick shape and attachment color.

Child collections are found through ``collections_all``; ``collections`` lists the root level only."""
from __future__ import annotations

try:
    import thyllore_avatar_api as avatar_api  # type: ignore
except ImportError:
    avatar_api = None  # type: ignore

CHAIN_COLLECTION_NAME = "Thyllore Chains"
CHAIN_BONE_DISPLAY_TYPE = "STICK"


def bone_palette(attachment: str | None) -> str:
    """The Blender bone color theme of a chain generated with ``attachment``; empty or None is an authored chain."""
    return f"THEME{avatar_api.chain_bone_palette(attachment or None):02d}"


def chain_bone_collection(armature_data):
    """The 'Thyllore Chains' bone collection that holds one child collection per chain, created on first use."""
    collection = armature_data.collections.get(CHAIN_COLLECTION_NAME)
    if collection is None:
        collection = armature_data.collections.new(CHAIN_COLLECTION_NAME)
    return collection


def chain_collection(armature_data, chain_name: str):
    """The bone collection named after the chain, under 'Thyllore Chains', created on first use."""
    parent = chain_bone_collection(armature_data)
    collection = armature_data.collections_all.get(chain_name)
    if collection is None:
        collection = armature_data.collections.new(chain_name, parent=parent)
    return collection


def apply_chain_bone_display(arm, chain_name: str, bone_names: list[str], attachment: str | None) -> None:
    """Put the bones in the chain's collection, draw them as sticks and color them by attachment."""
    collection = chain_collection(arm.data, chain_name)
    palette = bone_palette(attachment)
    for bone_name in bone_names:
        bone = arm.data.bones.get(bone_name)
        if bone is None:
            continue
        collection.assign(bone)
        bone.display_type = CHAIN_BONE_DISPLAY_TYPE
        bone.color.palette = palette


def remove_chain_collections(arm, chain_names: list[str]) -> None:
    """Drop the bone collections of chains whose bones are gone."""
    for chain_name in chain_names:
        collection = arm.data.collections_all.get(chain_name)
        if collection is not None:
            arm.data.collections.remove(collection)


def apply_chain_display(arm) -> None:
    """Apply the chain bone display to every joint of every chain on the armature."""
    for chain in arm.data.thyllore_spring.chains:
        chain_bones = [joint.bone for joint in chain.joints] + ([chain.tail] if chain.tail else [])
        apply_chain_bone_display(arm, chain.name, chain_bones, chain.origin_attachment)
