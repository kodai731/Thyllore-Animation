"""Blender property groups for avatar setup (spring bones, submesh roles).

Registered on ``bpy.types.Armature`` (spring) and ``bpy.types.Object`` (submesh).
"""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.effect_properties import build_param_properties


def _spring_joint_annotations() -> dict[str, object]:
    """Property annotations for one spring joint item from the engine's UI params."""
    if not _TML_AVAILABLE:
        return {}
    annotations: dict[str, object] = {}
    for param in tml.spring_joint_ui_params():
        annotations.update(build_param_properties(param))
    return annotations


SPRING_JOINT_PARAM_NAMES: tuple[str, ...] = (
    tuple(p["name"] for p in tml.spring_joint_ui_params()) if _TML_AVAILABLE else ()
)


class THYLLORE_PG_NameItem(bpy.types.PropertyGroup):
    """A single name entry for a UIList (collider group members, chain collider groups)."""

    name: bpy.props.StringProperty(name="Name")


class THYLLORE_PG_SpringJoint(bpy.types.PropertyGroup):
    """One spring bone joint: bone reference + joint parameters from the engine."""

    bone: bpy.props.StringProperty(name="Bone")

    __annotations__.update(_spring_joint_annotations())


class THYLLORE_PG_SpringChain(bpy.types.PropertyGroup):
    """One spring bone chain: name, center bone, tail bone, joints, collider groups."""

    name: bpy.props.StringProperty(name="Name")
    center: bpy.props.StringProperty(name="Center Bone")
    enabled: bpy.props.BoolProperty(name="Enabled", default=True)
    tail: bpy.props.StringProperty(name="Tail Bone")
    joints: bpy.props.CollectionProperty(type=THYLLORE_PG_SpringJoint)
    collider_groups: bpy.props.CollectionProperty(type=THYLLORE_PG_NameItem)


class THYLLORE_PG_Collider(bpy.types.PropertyGroup):
    """One spring bone collider: name, bone, offset, shape, radius, tail, origin."""

    name: bpy.props.StringProperty(name="Name")
    bone: bpy.props.StringProperty(name="Bone")
    offset: bpy.props.FloatVectorProperty(
        name="Offset", subtype="TRANSLATION", default=(0.0, 0.0, 0.0)
    )
    has_tail: bpy.props.BoolProperty(name="Has Tail", default=False)
    shape: bpy.props.EnumProperty(
        name="Shape",
        items=[("sphere", "Sphere", ""), ("capsule", "Capsule", "")],
        default="sphere",
    )
    radius: bpy.props.FloatProperty(name="Radius", min=0.0, default=0.1)
    tail: bpy.props.FloatVectorProperty(
        name="Tail", subtype="TRANSLATION", default=(0.0, 0.0, 0.0)
    )
    origin: bpy.props.EnumProperty(
        name="Origin",
        items=[("authored", "Authored", ""), ("fitted", "Fitted", "")],
        default="authored",
    )


class THYLLORE_PG_ColliderGroup(bpy.types.PropertyGroup):
    """One collider group: name + list of collider names."""

    name: bpy.props.StringProperty(name="Name")
    colliders: bpy.props.CollectionProperty(type=THYLLORE_PG_NameItem)


class THYLLORE_PG_HumanoidOverride(bpy.types.PropertyGroup):
    """One humanoid bone override: role -> bone mapping."""

    role: bpy.props.StringProperty(name="Role")
    bone: bpy.props.StringProperty(name="Bone")


class THYLLORE_PG_SpringSettings(bpy.types.PropertyGroup):
    """Spring bone settings stored on the armature (bpy.types.Armature.thyllore_spring)."""

    collision_mode: bpy.props.EnumProperty(
        name="Collision Mode",
        items=[("vrm_compatible", "VRM Compatible", ""), ("segment_extended", "Segment Extended", "")],
        default="vrm_compatible",
    )
    chains: bpy.props.CollectionProperty(type=THYLLORE_PG_SpringChain)
    colliders: bpy.props.CollectionProperty(type=THYLLORE_PG_Collider)
    collider_groups: bpy.props.CollectionProperty(type=THYLLORE_PG_ColliderGroup)
    humanoid_overrides: bpy.props.CollectionProperty(type=THYLLORE_PG_HumanoidOverride)
    active_chain_index: bpy.props.IntProperty(name="Active Chain Index", default=-1)
    active_collider_index: bpy.props.IntProperty(name="Active Collider Index", default=-1)
    active_group_index: bpy.props.IntProperty(name="Active Group Index", default=-1)
    gizmo_mode: bpy.props.EnumProperty(
        name="Gizmo Mode",
        items=[("selected", "Selected", ""), ("all", "All", ""), ("none", "None", "")],
        default="selected",
    )
    show_collider_gizmos: bpy.props.BoolProperty(name="Show Collider Gizmos")


class THYLLORE_PG_SubmeshRole(bpy.types.PropertyGroup):
    """One submesh role assignment: material -> role."""

    material: bpy.props.StringProperty(name="Material")
    role: bpy.props.EnumProperty(
        name="Role",
        items=[
            ("skin", "Skin", ""),
            ("garment", "Garment", ""),
            ("hair", "Hair", ""),
            ("accessory", "Accessory", ""),
        ],
        default="skin",
    )
    source: bpy.props.EnumProperty(
        name="Source",
        items=[("guessed", "Guessed", ""), ("user", "User", "")],
        default="guessed",
    )


class THYLLORE_PG_HiddenTriangles(bpy.types.PropertyGroup):
    """Hidden triangles for one material; triangle indices are stored as an int array on each element's ID property."""

    material: bpy.props.StringProperty(name="Material")


class THYLLORE_PG_SubmeshSettings(bpy.types.PropertyGroup):
    """Submesh role settings stored on the mesh object (bpy.types.Object.thyllore_submesh)."""

    roles: bpy.props.CollectionProperty(type=THYLLORE_PG_SubmeshRole)
    hidden_triangles: bpy.props.CollectionProperty(type=THYLLORE_PG_HiddenTriangles)
    active_role_index: bpy.props.IntProperty(name="Active Role Index", default=-1)


class THYLLORE_PG_RampForm(bpy.types.PropertyGroup):
    """Ramp form: root/tip joint params + shape + scope."""

    root: bpy.props.PointerProperty(type=THYLLORE_PG_SpringJoint)
    tip: bpy.props.PointerProperty(type=THYLLORE_PG_SpringJoint)
    shape: bpy.props.EnumProperty(
        name="Shape",
        items=[("linear", "Linear", ""), ("ease_in", "Ease In", ""), ("ease_out", "Ease Out", "")],
        default="linear",
    )
    scope: bpy.props.EnumProperty(
        name="Scope",
        items=[
            ("this_chain", "This Chain", ""),
            ("sibling_chains", "Sibling Chains", ""),
            ("all_chains", "All Chains", ""),
        ],
        default="this_chain",
    )


CLASSES = (
    THYLLORE_PG_NameItem,
    THYLLORE_PG_SpringJoint,
    THYLLORE_PG_SpringChain,
    THYLLORE_PG_Collider,
    THYLLORE_PG_ColliderGroup,
    THYLLORE_PG_HumanoidOverride,
    THYLLORE_PG_SpringSettings,
    THYLLORE_PG_SubmeshRole,
    THYLLORE_PG_HiddenTriangles,
    THYLLORE_PG_SubmeshSettings,
    THYLLORE_PG_RampForm,
)


def register() -> None:
    for cls in CLASSES:
        bpy.utils.register_class(cls)

    bpy.types.Armature.thyllore_spring = bpy.props.PointerProperty(
        type=THYLLORE_PG_SpringSettings
    )
    bpy.types.Object.thyllore_submesh = bpy.props.PointerProperty(
        type=THYLLORE_PG_SubmeshSettings
    )
    bpy.types.WindowManager.thyllore_ramp = bpy.props.PointerProperty(
        type=THYLLORE_PG_RampForm
    )


def unregister() -> None:
    del bpy.types.WindowManager.thyllore_ramp
    del bpy.types.Object.thyllore_submesh
    del bpy.types.Armature.thyllore_spring

    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)

