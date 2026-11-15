extern crate self as thyllore_effect_core;

pub mod debug;
pub mod effect_facade;
pub mod flame;
pub mod gpu_pack;
pub mod lightning;
pub mod scene_convert;
pub mod volume;
pub mod water;
pub mod wind;

pub use thyllore_texture_fit_core as flame_fit;

pub use thyllore_scene_core::{
    find_scalar_param, find_ui_param, title_case_snake, ScalarParam, SceneComponent, UiParam,
};

pub use debug::flame_wall_probe::{
    probe_flame_wall, WallProbeRay, WallProbeReport, WallProbeView, WALL_PROBE_GRID_COLS,
    WALL_PROBE_GRID_ROWS,
};
pub use effect_facade::{EffectPresets, Placement};
pub use flame::{
    advance_flame_trail, apply_flame_preset, apply_flame_style, apply_texture_fit,
    build_flame_inverse_model_matrix, build_flame_ubo, build_flame_ubo_with_trail,
    build_warp_strain_params, envelope_fade, eroded_argument, erosion_remap_scale,
    fit_flame_coefficients, fit_flame_texture, flame_bounding_radius, flame_field_manifest,
    flame_local_bounds, flame_local_bounds_corners, flame_proxy_radial_pad,
    flame_shell_support_scale, flame_style_from_effect, intersect_flame_proxy,
    noise_sharpness_to_shaping_scale, read_env_unified_beta, read_env_unified_tilt_gain_b,
    read_env_unified_tilt_gain_w, read_env_warp_form_displacement, read_env_wave_unified,
    refresh_flame_coefficients, ring_support_span, shaping_scale_to_noise_sharpness,
    vortex_macro_parameters, FieldManifest, FieldSourceKind, FlameBaked, FlameBranchField,
    FlameCoefficients, FlameColor, FlameDebugView, FlameEdge, FlameEffect, FlameEnvelope,
    FlameNoise, FlameProfile, FlameProxyPad, FlameRadialTaper, FlameRenderSettings,
    FlameShadingMode, FlameStyle, FlameTemporalAccum, FlameTrailState, FlameUBO, FlameWarp,
    FlameWind, StyleGroups, TextureFitGroups, BRANCH_MAX_ELEMENTS, FLAME_MAX_INSTANCES,
    FLAME_PRESET_NAMES, FLAME_SCALAR_PARAMS, FLAME_UI_PARAMS, VORTEX_MACRO_MAX_GAIN,
    WAVE_SEGMENTS_MAX, WAVE_SEGMENTS_MIN,
};
pub use gpu_pack::UboPack;
pub use lightning::{
    apply_lightning_preset, build_lightning_ubo, burst_start_time, compute_lightning_segment_aabb,
    lightning_parameter_snapshot, LightningDebugView, LightningEffect, LightningRenderSettings,
    LightningSegmentsUBO, LightningShadingMode, LightningShape, LightningSource, LightningUBO,
    LIGHTNING_MAX_INSTANCES, LIGHTNING_MAX_WAYPOINTS, LIGHTNING_PRESET_NAMES,
    LIGHTNING_SCALAR_PARAMS, LIGHTNING_UI_PARAMS,
};
pub use thyllore_effect_derive::UboPack;
pub use volume::{
    clamp_ray_to_cone_frustum, RayKnots, RayPuffs, VolumeShell, PUFFS_PER_RAY, RAY_MAX_KNOTS,
};
pub use water::{
    absorption_to_transmitted_color, apply_water_preset, build_water_model_matrix, build_water_ubo,
    generate_water_wave_modes, inverse_view_proj_f64, pick_torus, transmitted_color_to_absorption,
    WaterRenderSettings, WaterSecondaryRays, WaterTemporalAccum, WaterTorusEffect, WaterUBO,
    ABSORPTION_REFERENCE_DISTANCE, WATER_MAX_INSTANCES, WATER_PRESET_NAMES, WATER_SCALAR_PARAMS,
    WATER_UI_PARAMS,
};
pub use wind::{
    apply_wind_preset, build_wind_model_matrix, build_wind_ubo, pick_wind,
    wind_local_bounds_corners, wind_parameter_snapshot, WindDebugView, WindRenderSettings,
    WindResolveScale, WindShadingMode, WindShadowSlot, WindTornadoEffect, WindUBO,
    WIND_MAX_INSTANCES, WIND_PRESET_NAMES, WIND_SCALAR_PARAMS, WIND_SHADOW_VOLUME_HEIGHT,
    WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS, WIND_SHADOW_VOLUME_THETA, WIND_UI_PARAMS,
};

pub use flame::analytic::pick as flame_pick;
pub use flame::analytic::radial as flame_radial;
pub use flame::analytic::shell as flame_shell;
pub use flame::analytic::wave as flame_wave;
pub use flame::bake::sdf as flame_sdf;
pub use flame::bake::texture_fit as flame_texture_fit;
pub use flame::plume as flame_plume;
pub use flame::trail as flame_trail;

#[cfg(test)]
mod analytic_manifest;
#[cfg(test)]
mod test_support;

#[cfg(any(feature = "python", feature = "python-test"))]
mod pybindings;
