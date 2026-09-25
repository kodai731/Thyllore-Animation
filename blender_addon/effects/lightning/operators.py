from __future__ import annotations

import bpy

from .properties import waypoint_local_points, waypoint_midpoint


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


class THYLLORE_OT_lightning_waypoint_add(bpy.types.Operator):

    bl_idname = "thyllore.lightning_waypoint_add"
    bl_label = "Add Waypoint"

    @classmethod
    def poll(cls, context):
        obj = context.active_object
        return (
            obj is not None
            and hasattr(obj, "thyllore_lightning")
            and obj.thyllore_lightning.is_lightning
        )

    def execute(self, context):
        import thyllore_effect_core as fx

        max_waypoints = fx.lightning_max_waypoints()
        obj = context.active_object
        waypoints = obj.thyllore_lightning_waypoints

        if len(waypoints) >= max_waypoints:
            self.report({"WARNING"}, f"Maximum of {max_waypoints} waypoints reached")
            return {"CANCELLED"}

        local_points = waypoint_local_points(obj)
        previous = tuple(local_points[-1]) if local_points else (0.0, 0.0, 0.0)
        midpoint = waypoint_midpoint(previous, tuple(obj.thyllore_lightning.end_offset))

        empty = bpy.data.objects.new(f"Waypoint {len(waypoints) + 1}", None)
        context.collection.objects.link(empty)
        empty.parent = obj
        empty.location = midpoint

        waypoints.add().target = empty

        return {"FINISHED"}


class THYLLORE_OT_lightning_waypoint_remove(bpy.types.Operator):

    bl_idname = "thyllore.lightning_waypoint_remove"
    bl_label = "Remove Waypoint"

    index: bpy.props.IntProperty(name="Index", default=0)

    @classmethod
    def poll(cls, context):
        obj = context.active_object
        return (
            obj is not None
            and hasattr(obj, "thyllore_lightning")
            and obj.thyllore_lightning.is_lightning
            and len(obj.thyllore_lightning_waypoints) > 0
        )

    def execute(self, context):
        obj = context.active_object
        waypoints = obj.thyllore_lightning_waypoints
        if 0 <= self.index < len(waypoints):
            waypoints.remove(self.index)
        return {"FINISHED"}
