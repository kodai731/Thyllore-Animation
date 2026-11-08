"""Body collider fit operator (thyllore.fit_body_colliders)."""
from __future__ import annotations

import bpy

try:
    import thyllore_avatar_api as avatar_api  # type: ignore

    _AVATAR_API_AVAILABLE = True
except ImportError:
    avatar_api = None  # type: ignore
    _AVATAR_API_AVAILABLE = False

from ..common.chain_bones import settings_from_properties
from ..common.effect_properties import build_param_properties
from ..common.rig_snapshot import RigSnapshot, read_bones, read_rig
from ..common.spring_properties import (
    spring_settings_to_edit,
    write_sidecar_spring,
)
from ..common.logged_report import report
from .operators import target_armature
from .ramp_operators import active_bone_name

CAPSULE_FIT_UI_PARAMS = avatar_api.capsule_fit_ui_params() if _AVATAR_API_AVAILABLE else []


def _setting_annotations() -> dict[str, object]:
    annotations: dict[str, object] = {}
    for param in CAPSULE_FIT_UI_PARAMS:
        annotations.update(build_param_properties(param))
    return annotations


def default_fit_settings() -> dict[str, float]:
    return {param["name"]: float(param["default"]) for param in CAPSULE_FIT_UI_PARAMS}


def fit_body_colliders(rig: RigSnapshot, fit_settings: dict[str, float]) -> int:
    """Replace the rig's fitted body colliders from its skin meshes; returns the collider count."""
    settings = rig.arm.data.thyllore_spring
    if not rig.skins:
        raise ValueError("No skin mesh found (no bound mesh has role 'skin')")

    bones = rig.bones
    sidecar = avatar_api.spring_colliders_to_model({"spring": spring_settings_to_edit(settings), "bones": bones})
    output = avatar_api.fit_body_colliders({
        "spring": sidecar, "bones": bones, "skins": rig.skins, "settings": fit_settings,
    })

    write_sidecar_spring(settings, avatar_api.spring_colliders_to_local({"spring": output, "bones": bones}))
    return len(output.get("colliders", []))


class THYLLORE_OT_FitBodyColliders(bpy.types.Operator):
    """Fit body colliders from skin mesh vertices."""

    bl_idname = "thyllore.fit_body_colliders"
    bl_label = "Fit Body Colliders"
    bl_options = {"REGISTER", "UNDO"}

    __annotations__ = _setting_annotations()

    @classmethod
    def poll(cls, context):
        return _AVATAR_API_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        try:
            collider_count = fit_body_colliders(
                read_rig(context, arm), settings_from_properties(CAPSULE_FIT_UI_PARAMS, self)
            )
        except ValueError as e:
            report(self, "ERROR", f"fit_body_colliders failed: {e}")
            return {"CANCELLED"}

        report(self, "INFO", f"Fitted {collider_count} body colliders")
        return {"FINISHED"}

def _unique_name(existing_names, base_name):
    """Generate a unique name by appending .001, .002, etc. if base_name already exists."""
    if base_name not in existing_names:
        return base_name
    counter = 1
    while f"{base_name}.{counter:03d}" in existing_names:
        counter += 1
    return f"{base_name}.{counter:03d}"


class THYLLORE_OT_AddCollider(bpy.types.Operator):
    """Add a collider to the active bone."""

    bl_idname = "thyllore.add_collider"
    bl_label = "Add Collider"
    bl_options = {"REGISTER", "UNDO"}

    shape: bpy.props.EnumProperty(
        name="Shape",
        items=[("sphere", "Sphere", ""), ("capsule", "Capsule", "")],
        default="sphere",
    )
    radius: bpy.props.FloatProperty(name="Radius", default=0.05, min=0.0)

    @classmethod
    def poll(cls, context):
        return _AVATAR_API_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        settings = arm.data.thyllore_spring

        bone_name = active_bone_name(context)
        if bone_name is None:
            report(self, "ERROR", "No active pose bone")
            return {"CANCELLED"}

        base_name = f"{bone_name}/Collider"
        existing_names = {c.name for c in settings.colliders}
        name = _unique_name(existing_names, base_name)

        collider = settings.colliders.add()
        collider.name = name
        collider.bone = bone_name
        collider.shape = self.shape
        collider.radius = self.radius
        collider.offset = (0.0, 0.0, 0.0)
        collider.origin = "authored"
        if self.shape == "capsule":
            collider.has_tail = True
            collider.tail = (0.0, self.radius * 2, 0.0)

        report(self, "INFO", f"Added collider '{name}' to bone '{bone_name}'")
        return {"FINISHED"}


class THYLLORE_OT_RemoveCollider(bpy.types.Operator):
    """Remove the active collider."""

    bl_idname = "thyllore.remove_collider"
    bl_label = "Remove Collider"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        arm = target_armature(context)
        if arm is None:
            return False
        settings = arm.data.thyllore_spring
        index = settings.active_collider_index
        return 0 <= index < len(settings.colliders)

    def execute(self, context):
        arm = target_armature(context)
        settings = arm.data.thyllore_spring
        index = settings.active_collider_index

        collider = settings.colliders[index]
        name_to_remove = collider.name

        for group in settings.collider_groups:
            i = 0
            while i < len(group.colliders):
                entry = group.colliders[i]
                if entry.name == name_to_remove:
                    group.colliders.remove(i)
                else:
                    i += 1

        settings.colliders.remove(index)

        if index >= len(settings.colliders):
            settings.active_collider_index = max(0, len(settings.colliders) - 1)

        report(self, "INFO", f"Removed collider '{name_to_remove}'")
        return {"FINISHED"}


class THYLLORE_OT_ApplyColliderGroups(bpy.types.Operator):
    """Apply assigned collider groups to spring chains."""

    bl_idname = "thyllore.apply_collider_groups"
    bl_label = "Apply Collider Groups"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return _AVATAR_API_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        settings = arm.data.thyllore_spring

        bone_name = active_bone_name(context)
        if bone_name is None:
            report(self, "ERROR", "No active pose bone")
            return {"CANCELLED"}

        spring = spring_settings_to_edit(settings)
        bones = read_bones(context, arm)
        chains = avatar_api.chains_under_bone({"spring": spring, "bones": bones, "bone_name": bone_name})

        if not chains:
            report(self, "WARNING", f"No chains found for bone '{bone_name}'")
            return {"CANCELLED"}

        groups = [g.name for g in settings.collider_groups if g.assigned]
        if not groups:
            report(self, "WARNING", "No collider groups are assigned")
            return {"CANCELLED"}

        try:
            output = avatar_api.assign_collider_groups({
                "spring": spring, "bones": bones, "chain_names": chains, "group_names": groups,
            })
        except ValueError as e:
            report(self, "ERROR", f"assign_collider_groups failed: {e}")
            return {"CANCELLED"}

        write_sidecar_spring(settings, output)

        assigned_names = set(groups)
        for g in settings.collider_groups:
            if g.name in assigned_names:
                g.assigned = True

        report(self, "INFO", f"Applied {len(groups)} collider groups to {len(chains)} chains")
        return {"FINISHED"}


OPERATORS = (
    THYLLORE_OT_FitBodyColliders,
    THYLLORE_OT_AddCollider,
    THYLLORE_OT_RemoveCollider,
    THYLLORE_OT_ApplyColliderGroups,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
