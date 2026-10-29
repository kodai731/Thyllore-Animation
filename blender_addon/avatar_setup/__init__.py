"""Avatar setup property groups (spring bones, submesh roles, collider gizmo)."""
from __future__ import annotations

from . import (
    chain_operators,
    collider_operators,
    gizmo_draw,
    operators,
    panels,
    properties,
    ramp_operators,
    submesh_operators,
    submesh_roles,
)


def register() -> None:
    properties.register()
    operators.register()
    chain_operators.register()
    collider_operators.register()
    ramp_operators.register()
    submesh_operators.register()
    submesh_roles.register()
    panels.register()
    gizmo_draw.register()


def unregister() -> None:
    gizmo_draw.unregister()
    panels.unregister()
    submesh_roles.unregister()
    submesh_operators.unregister()
    ramp_operators.unregister()
    collider_operators.unregister()
    chain_operators.unregister()
    operators.unregister()
    properties.unregister()
