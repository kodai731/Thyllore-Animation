from __future__ import annotations

from ._common import coordinates, effect_properties

precision_from_format = effect_properties.precision_from_format
property_kind = effect_properties.property_kind
collect_params = effect_properties.collect_params
merge_preset_params = effect_properties.merge_preset_params
select_exposed_params = effect_properties.select_exposed_params


def build_waypoint_property_group():
    import bpy

    return type(
        "ThylloreLightningWaypoint",
        (bpy.types.PropertyGroup,),
        {"__annotations__": {"target": bpy.props.PointerProperty(type=bpy.types.Object)}, "__module__": __name__},
    )


def waypoint_midpoint(previous, end):
    return tuple((a + b) * 0.5 for a, b in zip(previous, end))


def waypoint_local_points(lightning_obj):
    world_to_local = lightning_obj.matrix_world.inverted()
    return [
        world_to_local @ waypoint.target.matrix_world.translation
        for waypoint in lightning_obj.thyllore_lightning_waypoints
        if waypoint.target is not None
    ]


def target_local_point(lightning_obj):
    target = lightning_obj.thyllore_lightning_target
    if target is None:
        return None
    return lightning_obj.matrix_world.inverted() @ target.matrix_world.translation


def lightning_local_end_point(lightning_obj):
    target_point = target_local_point(lightning_obj)
    if target_point is not None:
        return tuple(target_point)
    return tuple(lightning_obj.thyllore_lightning.end_offset)


def lightning_default_preset() -> str:
    import thyllore_effect_core as fx

    return fx.lightning_preset_names()[0]


def lightning_render_params(lightning_obj) -> dict:
    import thyllore_effect_core as fx

    params = effect_properties.render_params(lightning_obj.thyllore_lightning, fx.lightning_preset_params)
    target_point = target_local_point(lightning_obj)
    if target_point is not None:
        params["end_offset"] = [float(v) for v in coordinates.blender_to_engine_point(target_point)]
    return params


def build_lightning_property_group():
    import thyllore_effect_core as fx

    return effect_properties.build_effect_property_group(
        ui_params=fx.lightning_ui_params,
        preset_params=fx.lightning_preset_params,
        preset_names=fx.lightning_preset_names,
        flag_name="is_lightning",
        default_preset=lightning_default_preset(),
        class_name="ThylloreLightningProperties",
        module_name=__name__,
    )
