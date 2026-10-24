"""Bone collection assignment helpers."""


def assign_bones_to_chain_collection(arm, bone_names: list[str]) -> None:
    """Assign bones by name to the 'Thyllore Chains' collection (creating it if needed)."""
    collection = arm.data.collections.get("Thyllore Chains")
    if collection is None:
        collection = arm.data.collections.new("Thyllore Chains")
    for bone_name in bone_names:
        bone = arm.data.bones.get(bone_name)
        if bone is not None:
            collection.assign(bone)
