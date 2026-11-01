"""Blender-side display of spring chain bones: collection, stick shape and role color."""

CHAIN_COLLECTION_NAME = "Thyllore Chains"
CHAIN_BONE_DISPLAY_TYPE = "STICK"
ROLE_PALETTES = {"hair": "THEME01", "garment": "THEME04", "accessory": "THEME05"}
DEFAULT_PALETTE = "THEME06"


def chain_bone_collection(armature_data):
    """The 'Thyllore Chains' bone collection, created on first use."""
    collection = armature_data.collections.get(CHAIN_COLLECTION_NAME)
    if collection is None:
        collection = armature_data.collections.new(CHAIN_COLLECTION_NAME)
    return collection


def apply_chain_bone_display(arm, bone_names: list[str], role: str) -> None:
    """Put the bones in the chain collection, draw them as sticks and color them by role."""
    collection = chain_bone_collection(arm.data)
    palette = ROLE_PALETTES.get(role, DEFAULT_PALETTE)
    for bone_name in bone_names:
        bone = arm.data.bones.get(bone_name)
        if bone is None:
            continue
        collection.assign(bone)
        bone.display_type = CHAIN_BONE_DISPLAY_TYPE
        bone.color.palette = palette


def apply_chain_display(arm) -> None:
    """Apply the chain bone display to every joint of every chain on the armature."""
    for chain in arm.data.thyllore_spring.chains:
        apply_chain_bone_display(arm, [joint.bone for joint in chain.joints], chain.origin_role)
