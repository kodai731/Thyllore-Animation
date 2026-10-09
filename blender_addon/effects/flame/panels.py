from __future__ import annotations

import bpy

from . import debug_tools
from ._common import effect_properties


class VIEW3D_PT_thyllore_flame(bpy.types.Panel):

    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Thyllore"
    bl_label = "Flame"

    @classmethod
    def poll(cls, context):
        return True

    def draw(self, context):
        obj = context.view_layer.objects.active
        if obj is None or not obj.thyllore_flame.is_flame:
            self.layout.operator("thyllore.flame_add")
            return

        layout = self.layout
        props = obj.thyllore_flame

        layout.prop(props, "preset")
        effect_properties.draw_primary_params(layout, props)


VIEW3D_PT_thyllore_flame_advanced = effect_properties.build_effect_advanced_panel(
    "flame", "VIEW3D_PT_thyllore_flame"
)


def _debug_draw(layout, props):
    debug_tools.draw_panel(layout)


VIEW3D_PT_thyllore_flame_debug = effect_properties.build_effect_child_panel(
    "flame", "VIEW3D_PT_thyllore_flame", "debug", "Debug", _debug_draw
)
