"""Avatar setup property groups (spring bones, submesh roles, collider gizmo)."""
from __future__ import annotations

from . import chain_operators, operators, panels, properties, ramp_operators


def register() -> None:
    properties.register()
    operators.register()
    chain_operators.register()
    ramp_operators.register()
    panels.register()


def unregister() -> None:
    panels.unregister()
    ramp_operators.unregister()
    chain_operators.unregister()
    operators.unregister()
    properties.unregister()
