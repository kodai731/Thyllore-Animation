"""Armature lookup shared by the chain and avatar setup operators."""
from __future__ import annotations


def armature_of(obj):
    """The armature a mesh object is bound to through its first ARMATURE modifier, or None."""
    for mod in obj.modifiers:
        if mod.type == "ARMATURE":
            return mod.object
    return None
