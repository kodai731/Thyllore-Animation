from __future__ import annotations

import bpy

from . import debug_tools
from ._common import effect_properties


def draw_target_row(layout, obj):
    target = obj.thyllore_lightning_target
    if target is None:
        layout.operator("thyllore.lightning_target_add")
        return
    row = layout.row()
    row.label(text=f"Target: {target.name}")
    row.operator("thyllore.lightning_target_clear", text="", icon="X")


class VIEW3D_PT_thyllore_lightning(bpy.types.Panel):

    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Thyllore"
    bl_label = "Lightning"

    @classmethod
    def poll(cls, context):
        return True

    def draw(self, context):
        obj = context.view_layer.objects.active
        if obj is None or not obj.thyllore_lightning.is_lightning:
            self.layout.operator("thyllore.lightning_add")
            return

        layout = self.layout
        props = obj.thyllore_lightning

        layout.prop(props, "preset")

        path_box = layout.box()
        path_box.label(text="Path")
        draw_target_row(path_box, obj)
        for index, waypoint in enumerate(obj.thyllore_lightning_waypoints):
            row = path_box.row()
            row.label(text=waypoint.target.name if waypoint.target is not None else "(missing)")
            row.operator("thyllore.lightning_waypoint_remove", text="", icon="X").index = index
        path_box.operator("thyllore.lightning_waypoint_add")

        effect_properties.draw_primary_params(layout, props)


VIEW3D_PT_thyllore_lightning_advanced = effect_properties.build_effect_advanced_panel(
    "lightning", "VIEW3D_PT_thyllore_lightning"
)


def _debug_draw(layout, props):
    debug_tools.draw_panel(layout)


VIEW3D_PT_thyllore_lightning_debug = effect_properties.build_effect_child_panel(
    "lightning", "VIEW3D_PT_thyllore_lightning", "debug", "Debug", _debug_draw
)
