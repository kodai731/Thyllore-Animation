from __future__ import annotations

import bpy


class THYLLORE_OT_lightning_add(bpy.types.Operator):

    bl_idname = "thyllore.lightning_add"
    bl_label = "Add Lightning"

    @classmethod
    def poll(cls, context):
        return context.mode == "OBJECT"

    def execute(self, context):
        from .properties import lightning_default_preset

        cursor_location = context.scene.cursor.location

        obj = bpy.data.objects.new("Lightning", None)
        obj.empty_display_type = "CUBE"
        obj.empty_display_size = 0.5
        obj.location = cursor_location

        context.collection.objects.link(obj)

        obj.thyllore_lightning.is_lightning = True
        obj.thyllore_lightning.preset = lightning_default_preset()

        context.view_layer.objects.active = obj
        obj.select_set(True)

        return {"FINISHED"}
