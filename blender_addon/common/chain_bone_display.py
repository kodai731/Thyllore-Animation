"""Blender-side display of spring chain bones: one bone collection per chain, stick shape and role color.

Child collections are found through ``collections_all``; ``collections`` lists the root level only."""

CHAIN_COLLECTION_NAME = "Thyllore Chains"
CHAIN_BONE_DISPLAY_TYPE = "STICK"
ROLE_PALETTES = {"hair": "THEME01", "garment": "THEME04", "accessory": "THEME05"}
DEFAULT_PALETTE = "THEME06"


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


def apply_chain_bone_display(arm, chain_name: str, bone_names: list[str], role: str) -> None:
    """Put the bones in the chain's collection, draw them as sticks and color them by role."""
    collection = chain_collection(arm.data, chain_name)
    palette = ROLE_PALETTES.get(role, DEFAULT_PALETTE)
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
        apply_chain_bone_display(arm, chain.name, [joint.bone for joint in chain.joints], chain.origin_role)
