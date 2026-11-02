"""Pure Python helpers for reading/writing spring bone settings from Blender property groups."""
from __future__ import annotations

from .coordinates import blender_to_engine_point, engine_to_blender_point

FORCE_PARAM_NAMES: dict[str, tuple[str, ...]] = {}


def configure_force_params(names_by_model: dict[str, tuple[str, ...]]) -> None:
    """Register the magnitude names of every force model, as the engine declares them."""
    FORCE_PARAM_NAMES.clear()
    FORCE_PARAM_NAMES.update(names_by_model)


def spring_settings_to_sidecar(settings) -> dict:
    """Read spring settings from a Blender property group and build the sidecar dict.

    Only enabled chains are included — disabled chains are excluded from export."""
    return _build_sidecar_from_chains(
        [c for c in settings.chains if c.enabled],
        list(settings.colliders),
        list(settings.collider_groups),
        settings.collision_mode,
    )


def spring_settings_to_edit(settings) -> dict:
    """Build the sidecar spring dict from Blender spring settings (edit view).

    Includes ALL chains regardless of enabled state — used for edit operators that
    round-trip through write_sidecar_spring."""
    return _build_sidecar_from_chains(
        list(settings.chains),
        list(settings.colliders),
        list(settings.collider_groups),
        settings.collision_mode,
    )


def disabled_chain_names(settings) -> set[str]:
    """Return the names of chains that are currently disabled."""
    return {c.name for c in settings.chains if not c.enabled}


def restore_disabled_chains(settings, names: set[str]) -> None:
    """Restore disabled state: set enabled=False for chains whose names are in *names*."""
    for chain in settings.chains:
        if chain.name in names:
            chain.enabled = False

def _build_sidecar_from_chains(chains, colliders, groups, collision_mode) -> dict:
    """Build the sidecar spring dict from explicit lists of chains/colliders/groups.

    This is the shared private implementation — callers pass whichever chains they need."""
    return {
        "collision_mode": collision_mode,
        "chains": [_chain_to_sidecar(chain) for chain in chains],
        "colliders": [_collider_to_sidecar(collider) for collider in colliders],
        "collider_groups": [_collider_group_to_sidecar(group) for group in groups],
    }


def _chain_to_sidecar(chain) -> dict:
    """Build sidecar dict for one chain item."""
    return {
        "name": chain.name,
        "center": chain.center or None,
        "collider_groups": [entry.name for entry in chain.collider_groups],
        "joints": [_joint_to_sidecar(joint) for joint in chain.joints],
        "tail": chain.tail or None,
        "origin": _chain_origin_to_sidecar(chain),
    }


def _chain_origin_to_sidecar(chain) -> dict | None:
    """The submesh a generated chain came from, or None for an authored chain."""
    mesh = getattr(chain, "origin_mesh", "")
    if not mesh:
        return None
    return {
        "mesh": mesh,
        "material": chain.origin_material,
        "role": chain.origin_role,
    }


def _joint_to_sidecar(joint) -> dict:
    """Build sidecar dict for one joint item."""
    return {"bone": joint.bone, **joint_values(joint)}


def _collider_to_sidecar(collider) -> dict:
    """Build sidecar dict for one collider item; offset and tail stay in the bone's rest frame, the
    operator moves them into model space through ``spring_colliders_to_model``."""
    tail_value = None
    if collider.has_tail and collider.tail is not None:
        tail_value = list(collider.tail)
    return {
        "name": collider.name,
        "bone": collider.bone,
        "offset": list(collider.offset),
        "shape": collider.shape,
        "radius": float(collider.radius),
        "tail": tail_value,
        "origin": collider.origin,
    }


def _collider_group_to_sidecar(group) -> dict:
    """Build sidecar dict for one collider group item."""
    return {
        "name": group.name,
        "colliders": [entry.name for entry in group.colliders],
    }


def write_sidecar_spring(settings, spring: dict) -> None:
    """Write sidecar spring data into a Blender property group (idempotent)."""
    settings.collision_mode = spring["collision_mode"]

    _write_chains(settings.chains, spring["chains"])
    _write_colliders(settings.colliders, spring["colliders"])
    _write_collider_groups(settings.collider_groups, spring["collider_groups"])


def _write_chains(chains, spring_chains: list[dict]) -> None:
    chains.clear()
    for chain in spring_chains:
        item = chains.add()
        item.name = chain["name"]
        item.center = chain.get("center") or ""
        item.enabled = True
        item.tail = chain.get("tail") or ""
        origin = chain.get("origin") or {}
        item.origin_mesh = origin.get("mesh", "")
        item.origin_material = origin.get("material", "")
        item.origin_role = origin.get("role", "")
        for joint in chain["joints"]:
            _write_joint(item.joints, joint)
        for group_name in chain.get("collider_groups", []):
            entry = item.collider_groups.add()
            entry.name = group_name


def _write_joint(joints_collection, joint: dict) -> None:
    """Write one joint dict into a Blender joints collection."""
    joint_item = joints_collection.add()
    joint_item.bone = joint["bone"]
    write_joint_force(joint_item, joint["force"])
    joint_item.drag_force = joint["drag_force"]
    joint_item.gravity_dir = engine_to_blender_point(tuple(joint["gravity_dir"]))
    joint_item.hit_radius = joint["hit_radius"]


def _write_colliders(colliders, spring_colliders: list[dict]) -> None:
    colliders.clear()
    for collider in spring_colliders:
        collider_item = colliders.add()
        collider_item.name = collider["name"]
        collider_item.bone = collider["bone"]
        collider_item.offset = tuple(collider["offset"])
        collider_item.shape = collider["shape"]
        collider_item.radius = collider["radius"]
        if collider.get("tail") is not None:
            collider_item.has_tail = True
            collider_item.tail = tuple(collider["tail"])
        else:
            collider_item.has_tail = False
            collider_item.tail = (0.0, 0.0, 0.0)
        collider_item.origin = collider.get("origin", "authored")


def _write_collider_groups(collider_groups, spring_groups: list[dict]) -> None:
    collider_groups.clear()
    for group in spring_groups:
        group_item = collider_groups.add()
        group_item.name = group["name"]
        for collider_name in group["colliders"]:
            entry = group_item.colliders.add()
            entry.name = collider_name


def humanoid_overrides_to_dict(settings) -> dict[str, str]:
    """Read humanoid bone overrides from a Blender property group."""
    result = {}
    for item in settings.humanoid_overrides:
        result[item.role] = item.bone
    return result


def joint_force_values(joint) -> dict:
    """The joint's force in the sidecar's shape: the model and that model's magnitudes."""
    model = joint.force_model
    force = {"model": model}
    for name in FORCE_PARAM_NAMES[model]:
        force[name] = float(getattr(joint, name))
    return force


def write_joint_force(joint_item, force: dict) -> None:
    joint_item.force_model = force["model"]
    for name in FORCE_PARAM_NAMES[force["model"]]:
        setattr(joint_item, name, force[name])


def joint_values(joint) -> dict:
    """Read the joint param values from a Blender spring joint item (without bone)."""
    return {
        "force": joint_force_values(joint),
        "drag_force": float(joint.drag_force),
        "gravity_dir": blender_to_engine_point(tuple(joint.gravity_dir)),
        "hit_radius": float(joint.hit_radius),
    }


def ramp_values(form) -> dict:
    """Read the ramp form (root / tip joints, rigid fraction, shape) as the apply_ramp input."""
    return {
        "root": joint_values(form.root),
        "tip": joint_values(form.tip),
        "shape": form.shape,
        "rigid_fraction": float(form.rigid_fraction),
    }


def copy_joint_values(source, target) -> None:
    """Copy joint param values from source to target Blender spring joint item: the force model
    with every model's magnitudes, drag_force, gravity_dir, hit_radius."""
    target.force_model = source.force_model
    for names in FORCE_PARAM_NAMES.values():
        for name in names:
            setattr(target, name, getattr(source, name))
    target.drag_force = source.drag_force
    target.gravity_dir = source.gravity_dir
    target.hit_radius = source.hit_radius


def _chain_joint_pairs(settings):
    """Yield (chain, joint) for every joint in every chain."""
    for chain in settings.chains:
        for joint in chain.joints:
            yield chain, joint


def joint_for_bone(settings, bone_name: str):
    """Find the first joint item across all chains matching bone_name."""
    for _, joint in _chain_joint_pairs(settings):
        if joint.bone == bone_name:
            return joint
    return None


def chain_with_joint(settings, bone_name: str):
    """Find the first chain item that has a joint matching bone_name."""
    for chain, joint in _chain_joint_pairs(settings):
        if joint.bone == bone_name:
            return chain
    return None


def same_origin(chain, origin: dict) -> bool:
    """Whether a chain item was generated on the submesh ``origin`` names (mesh, material)."""
    return (
        bool(getattr(chain, "origin_mesh", ""))
        and chain.origin_mesh == origin["mesh"]
        and chain.origin_material == (origin.get("material") or "")
    )


def chains_generated_on(settings, origin: dict) -> list[str]:
    """Names of the chains generated on ``origin``'s submesh; when its material matches none, every
    chain generated on that mesh."""
    exact = [chain.name for chain in settings.chains if same_origin(chain, origin)]
    if exact:
        return exact
    return [chain.name for chain in settings.chains if getattr(chain, "origin_mesh", "") == origin["mesh"]]


def active_chain(settings):
    """The chain selected in the Chains list, or None when nothing is selected."""
    index = settings.active_chain_index
    if 0 <= index < len(settings.chains):
        return settings.chains[index]
    return None


def remove_chains_generated_on(settings, origin: dict) -> list[str]:
    """Drop the chains generated on ``origin`` from the settings; returns their joint bone names."""
    bone_names: list[str] = []
    for index in reversed(range(len(settings.chains))):
        chain = settings.chains[index]
        if not same_origin(chain, origin):
            continue
        bone_names.extend(joint.bone for joint in chain.joints)
        settings.chains.remove(index)
    return bone_names
