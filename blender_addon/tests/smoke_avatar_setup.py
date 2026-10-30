"""Blender headless smoke test for avatar setup pipeline.

Invoked from run_avatar_setup_smoke.sh:
    blender --background --factory-startup -noaudio --python-exit-code 1 \
        --python blender_addon/tests/smoke_avatar_setup.py

Each step function raises SystemExit on failure; the script prints
"AVATAR_SETUP_SMOKE ok" at the end.
"""
from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _bootstrap() -> None:
    """Insert wheels-extracted paths into sys.path and register avatar_setup."""
    import bpy

    sys.path.insert(0, str(REPO_ROOT))

    extracted = REPO_ROOT / "blender_addon" / "wheels-extracted"
    if extracted.is_dir():
        for wheel_dir in sorted(extracted.iterdir()):
            if wheel_dir.is_dir() and str(wheel_dir) not in sys.path:
                sys.path.insert(0, str(wheel_dir))

    bpy.ops.wm.read_factory_settings(use_empty=True)

    from blender_addon import avatar_setup
    from blender_addon import operators

    avatar_setup.register()
    operators.register()


def step_a_import_fbx():
    """(a) Import assets/models/fuse-woman-1/source/woman 1.fbx."""
    import bpy

    fbx_path = str(REPO_ROOT / "assets" / "models" / "fuse-woman-1" / "source" / "woman 1.fbx")
    bpy.ops.import_scene.fbx(filepath=fbx_path)

    arm = None
    mesh = None
    for obj in bpy.data.objects:
        if obj.type == "ARMATURE":
            arm = obj
        elif obj.type == "MESH" and mesh is None:
            mesh = obj

    if arm is None or mesh is None:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_a — no armature or mesh after FBX import")

    print(f"[smoke] step_a: imported {mesh.name} ({len(mesh.data.vertices)} verts) + {arm.name}")
    return arm, mesh


def step_b_guess_submesh_roles(arm, mesh):
    """(b) Armature active -> thyllore.guess_submesh_roles -> Body=skin / Hair=hair."""
    import bpy

    bpy.context.view_layer.objects.active = arm
    bpy.ops.thyllore.guess_submesh_roles()

    submesh = mesh.thyllore_submesh
    roles = {item.material: item.role for item in submesh.roles}

    body_material = None
    hair_material = None
    for mat_name, role in roles.items():
        if "Body" in mat_name:
            body_material = mat_name
        if "Hair" in mat_name:
            hair_material = mat_name

    if body_material is None:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_b — no Body material found")
    if hair_material is None:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_b — no Hair material found")

    if roles[body_material] != "skin":
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_b — Body role is {roles[body_material]!r}, expected 'skin'"
        )
    if roles[hair_material] != "hair":
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_b — Hair role is {roles[hair_material]!r}, expected 'hair'"
        )

    print(f"[smoke] step_b: {body_material}={roles[body_material]}, {hair_material}={roles[hair_material]}")
    return hair_material


def step_c_generate_chain_bones(arm, mesh, hair_material):
    """(c) Generate chain bones on hair submesh -> added > 0, multiple of joint_count (4), all in Thyllore Chains."""
    import bpy

    bone_count_before = len(arm.data.bones)

    hair_slot_index = None
    for i, slot in enumerate(mesh.material_slots):
        if slot.material is not None and slot.material.name == hair_material:
            hair_slot_index = i
            break

    if hair_slot_index is None:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_c — Hair material slot not found")

    bpy.context.view_layer.objects.active = mesh
    bpy.ops.thyllore.generate_chain_bones(
        role="hair",
        material_index=hair_slot_index,
        root_bone="mixamorig:Head",
        joint_count=4,
    )

    bone_count_after = len(arm.data.bones)
    added = bone_count_after - bone_count_before

    if added <= 0:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_c — expected > 0 chain bones, got {added}"
        )
    if added % 4 != 0:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_c — added {added} bones is not a multiple of joint_count (4)"
        )

    collection = arm.data.collections.get("Thyllore Chains")
    if collection is None:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_c — 'Thyllore Chains' bone collection missing")
    if len(collection.bones) != added:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_c — 'Thyllore Chains' has {len(collection.bones)} bones, expected {added}"
        )

    print(f"[smoke] step_c: {added} chain bones added, all in 'Thyllore Chains' collection")
    return added


def step_d_generated_chains_are_registered(arm, added):
    """(d) Generate Chain Bones registered one spring chain per 4 bones; Add Chains Under Bone adds none more."""
    import bpy

    settings = arm.data.thyllore_spring
    expected = added // 4
    chain_count = len(settings.chains)
    if chain_count != expected:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_d — expected {expected} spring chains from generation (added={added} / 4), got {chain_count}"
        )
    if any(not chain.origin_mesh for chain in settings.chains):
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_d — a generated chain has no origin")
    radii = [joint.hit_radius for chain in settings.chains for joint in chain.joints]
    if max(radii) <= 0.0:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_d — no generated joint has a measured hit_radius")

    head_colliders = [c for c in settings.colliders if c.bone == "mixamorig:Head" and c.shape == "capsule"]
    if len(head_colliders) != 1:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_d — expected one head capsule collider, got {len(head_colliders)}"
        )
    fitted = [g for g in settings.collider_groups if g.name == "Fitted Body"]
    if len(fitted) != 1 or any(
        "Fitted Body" not in [entry.name for entry in chain.collider_groups] for chain in settings.chains
    ):
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_d — generated chains are not bound to 'Fitted Body'")
    print(f"[smoke] step_d: head capsule r={head_colliders[0].radius:.3f} on every generated chain")

    bpy.context.view_layer.objects.active = arm
    bpy.ops.thyllore.add_chains_under_bone(root_bone="mixamorig:Head", min_joints=2)
    chain_count_after = len(settings.chains)
    if chain_count_after != expected:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_d — add_chains_under_bone changed the chain count {expected} -> {chain_count_after}"
        )

    print(f"[smoke] step_d: {chain_count} spring chains registered by generation, idempotent under Head")


def step_e_fit_body_colliders(arm):
    """(e) thyllore.fit_body_colliders() -> 'Fitted Body' group on all chains."""
    import bpy

    bpy.context.view_layer.objects.active = arm
    bpy.ops.thyllore.fit_body_colliders()

    settings = arm.data.thyllore_spring
    groups = [g.name for g in settings.collider_groups]

    if "Fitted Body" not in groups:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_e — 'Fitted Body' group not found, groups={groups}"
        )

    fitted_group = None
    for g in settings.collider_groups:
        if g.name == "Fitted Body":
            fitted_group = g
            break

    if len(fitted_group.colliders) == 0:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_e — 'Fitted Body' has no colliders"
        )

    print(f"[smoke] step_e: 'Fitted Body' group with {len(fitted_group.colliders)} colliders")


def step_f_apply_ramp(arm):
    """(f) Head bone active -> thyllore.apply_ramp() returns FINISHED."""
    import bpy

    head_bone = arm.data.bones.get("mixamorig:Head")
    if head_bone is None:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_f — 'mixamorig:Head' bone not found")

    arm.data.bones.active = head_bone
    result = bpy.ops.thyllore.apply_ramp()

    if "FINISHED" not in result:
        raise SystemExit(
            f"AVATAR_SETUP_SMOKE failed: step_f — apply_ramp returned {result}, expected FINISHED"
        )

    print("[smoke] step_f: apply_ramp returned FINISHED")


def step_g_export_for_unity(arm, mesh):
    """(g) thyllore.export_for_unity -> FBX + woman1.avatar.json exist and are readable."""
    import bpy

    output_dir = REPO_ROOT / "log" / "blender_avatar_setup_smoke"
    output_dir.mkdir(parents=True, exist_ok=True)
    fbx_path = str(output_dir / "woman1.fbx")

    bpy.ops.thyllore.export_for_unity(filepath=fbx_path)

    fbx_file = Path(fbx_path)
    sidecar_file = fbx_file.with_suffix(".avatar.json")

    if not fbx_file.exists():
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_g — FBX file not created")
    if not sidecar_file.exists():
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_g — sidecar JSON not created")

    import thyllore_ml_core as tml

    text = sidecar_file.read_text(encoding="utf-8")
    try:
        sidecar = tml.sidecar_read(text)
    except (RuntimeError, ValueError) as e:
        raise SystemExit(f"AVATAR_SETUP_SMOKE failed: step_g — sidecar_read failed: {e}")

    bone_count = sidecar.get("stats", {}).get("bones", 0)
    if bone_count == 0:
        raise SystemExit("AVATAR_SETUP_SMOKE failed: step_g — sidecar has no bones")

    print(f"[smoke] step_g: exported {fbx_file} ({fbx_file.stat().st_size} bytes) + sidecar ({bone_count} bones)")


def main() -> None:
    _bootstrap()
    arm, mesh = step_a_import_fbx()
    hair_material = step_b_guess_submesh_roles(arm, mesh)
    added = step_c_generate_chain_bones(arm, mesh, hair_material)
    step_d_generated_chains_are_registered(arm, added)
    step_e_fit_body_colliders(arm)
    step_f_apply_ramp(arm)
    step_g_export_for_unity(arm, mesh)
    print("AVATAR_SETUP_SMOKE ok")


if __name__ == "__main__":
    main()
