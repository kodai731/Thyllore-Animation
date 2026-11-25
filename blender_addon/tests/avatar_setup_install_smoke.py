"""Headless check that an installed Thyllore Avatar Setup extension enables and registers.

Invoked after ``blender --command extension install-file -r user_default --enable <zip>``:
    blender --background -noaudio --python-exit-code 1 \
        --python blender_addon/tests/avatar_setup_install_smoke.py

Fails when the extension cannot be enabled, when an Avatar Setup operator is missing,
or when a curve copilot operator leaked into the avatar-setup product.
"""
from __future__ import annotations

PACKAGE = "bl_ext.user_default.thyllore_avatar_setup"
REQUIRED_OPERATORS = ("reset_vrm_meta",)
EXCLUDED_OPERATORS = ("curve_copilot",)


def enable_extension() -> None:
    import addon_utils
    import bpy

    loaded_default, loaded_state = addon_utils.check(PACKAGE)
    if loaded_default or loaded_state:
        return
    bpy.ops.preferences.addon_enable(module=PACKAGE)
    if PACKAGE not in bpy.context.preferences.addons:
        raise SystemExit(f"AVATAR_SETUP_INSTALL_SMOKE failed: {PACKAGE} did not enable")


def is_operator_registered(name: str) -> bool:
    import bpy

    return hasattr(bpy.types, f"THYLLORE_OT_{name}")


def check_operators() -> None:
    for name in REQUIRED_OPERATORS:
        if not is_operator_registered(name):
            raise SystemExit(f"AVATAR_SETUP_INSTALL_SMOKE failed: operator thyllore.{name} missing")
    for name in EXCLUDED_OPERATORS:
        if is_operator_registered(name):
            raise SystemExit(f"AVATAR_SETUP_INSTALL_SMOKE failed: operator thyllore.{name} shipped")


def main() -> None:
    enable_extension()
    check_operators()
    print("AVATAR_SETUP_INSTALL_SMOKE ok")


if __name__ == "__main__":
    main()
