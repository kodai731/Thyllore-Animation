from __future__ import annotations

from . import _bootstrap
from . import properties
from ._common import viewport_recording


def register():
    import bpy

    from . import debug_tools
    from . import draw_handler
    from . import operators
    from . import panels

    _bootstrap.insert_wheels_to_sys_path()

    cls = properties.build_lightning_property_group()
    properties._registered_cls = cls
    bpy.utils.register_class(cls)
    bpy.types.Object.thyllore_lightning = bpy.props.PointerProperty(type=cls)

    waypoint_cls = properties.build_waypoint_property_group()
    properties._registered_waypoint_cls = waypoint_cls
    bpy.utils.register_class(waypoint_cls)
    bpy.types.Object.thyllore_lightning_waypoints = bpy.props.CollectionProperty(type=waypoint_cls)

    bpy.utils.register_class(operators.THYLLORE_OT_lightning_add)
    bpy.utils.register_class(operators.THYLLORE_OT_lightning_waypoint_add)
    bpy.utils.register_class(operators.THYLLORE_OT_lightning_waypoint_remove)
    debug_tools.register()
    bpy.utils.register_class(panels.VIEW3D_PT_thyllore_lightning)
    bpy.utils.register_class(panels.VIEW3D_PT_thyllore_lightning_advanced)
    bpy.utils.register_class(panels.VIEW3D_PT_thyllore_lightning_debug)
    viewport_recording.register()

    draw_handler.register_draw_handler()


def unregister():
    import bpy

    from . import debug_tools
    from . import draw_handler
    from . import operators
    from . import panels

    draw_handler.unregister_draw_handler()
    viewport_recording.unregister()
    bpy.utils.unregister_class(panels.VIEW3D_PT_thyllore_lightning_debug)
    bpy.utils.unregister_class(panels.VIEW3D_PT_thyllore_lightning_advanced)
    bpy.utils.unregister_class(panels.VIEW3D_PT_thyllore_lightning)
    debug_tools.unregister()
    bpy.utils.unregister_class(operators.THYLLORE_OT_lightning_waypoint_remove)
    bpy.utils.unregister_class(operators.THYLLORE_OT_lightning_waypoint_add)
    bpy.utils.unregister_class(operators.THYLLORE_OT_lightning_add)

    del bpy.types.Object.thyllore_lightning_waypoints
    if hasattr(properties, "_registered_waypoint_cls"):
        bpy.utils.unregister_class(properties._registered_waypoint_cls)
        del properties._registered_waypoint_cls
    del bpy.types.Object.thyllore_lightning

    if hasattr(properties, "_registered_cls"):
        bpy.utils.unregister_class(properties._registered_cls)
        del properties._registered_cls

    _bootstrap.remove_wheels_from_sys_path()
