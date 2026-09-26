use super::scene_format::*;
use crate::lightning::{
    LightningBranching, LightningLook, LightningParameterOwner, LightningShape, LightningSource,
    LightningTiming,
};
use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(
    Clone, Debug, PartialEq, thyllore_effect_derive::SceneFormat, thyllore_effect_derive::PyEffect,
)]
#[py_effect(presets = crate::LIGHTNING_PRESET_NAMES, apply_preset = crate::apply_lightning_preset)]
#[scene(record = LightningSceneRecord, tag = LightningParameterOwner, key = "lightning", tags = LIGHTNING_PARAMETER_OWNERSHIP, snapshot = lightning_parameter_snapshot, scalars = LIGHTNING_SCALAR_PARAMS, ui = LIGHTNING_UI_PARAMS, overwrite = overwrite_lightning_persisted_fields)]
pub struct LightningEffect {
    #[persist(owner = Frame, as = [f32; 3], get = lightning_position_get, set = lightning_position_set)]
    pub position: Vector3<f32>,
    #[persist(owner = Frame, as = [f32; 4], get = lightning_rotation_get, set = lightning_rotation_set)]
    pub rotation: Quaternion<f32>,
    #[runtime(ui(min = 0.0, max = 100.0, format = "%.2f"))]
    pub time: f32,
    #[runtime(ui(primary, min = 0.0, max = 4.0, format = "%.2f"))]
    pub time_scale: f32,
    #[runtime(ui(min = -100.0, max = 100.0, format = "%.2f"))]
    pub time_offset: f32,
    #[persist(owner = Frame, name = source, as = LightningSource, get = lightning_source_get, set = lightning_source_set)]
    #[persist(owner = Frame, name = end_offset, path = "shape.end_offset", as = [f32; 3], scalars(end_offset_x = "shape.end_offset[0]", end_offset_y = "shape.end_offset[1]", end_offset_z = "shape.end_offset[2]"), ui(kind = Offset, min = -50.0, max = 50.0, format = "%.2f", tooltip = "End point of the main strike relative to the effect origin", group = "shape"))]
    #[persist(owner = Frame, name = strikes_per_burst, path = "shape.strikes_per_burst", as = u32, ui(min = 1.0, max = 32.0, format = "%.0f", group = "shape"))]
    #[persist(owner = Frame, name = detail_levels, path = "shape.detail_levels", as = u32, ui(min = 1.0, max = 8.0, format = "%.0f", tooltip = "Midpoint displacement subdivisions of the channel; the segment count is 2^detail_levels", group = "shape"))]
    #[persist(owner = Frame, name = tortuosity, path = "shape.tortuosity", as = f32, ui(primary, min = 0.0, max = 2.0, format = "%.2f", tooltip = "Lateral displacement of the first subdivision relative to the strike length", group = "shape"))]
    #[persist(owner = Frame, name = roughness, path = "shape.roughness", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Decay of the displacement per subdivision level; 0.5 is a Brownian channel", group = "shape"))]
    #[persist(owner = Frame, name = core_radius, path = "shape.core_radius", as = f32, ui(primary, min = 0.001, max = 1.0, format = "%.3f", group = "shape"))]
    #[persist(owner = Frame, name = tip_radius_ratio, path = "shape.tip_radius_ratio", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Core radius at the strike tip as a fraction of core_radius", group = "shape"))]
    #[persist(owner = Frame, name = edge_fraction, path = "shape.edge_fraction", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Fraction of the core radius over which the coverage falls to zero", group = "shape"))]
    pub shape: LightningShape,
    #[persist(owner = Frame, name = branch_depth, path = "branch.depth", as = u32, ui(min = 0.0, max = 5.0, format = "%.0f", tooltip = "Recursion depth of the branch tree; 0 leaves the main channel alone", group = "branch"))]
    #[persist(owner = Frame, name = branch_probability, path = "branch.probability", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", group = "branch"))]
    #[persist(owner = Frame, name = branch_count, path = "branch.count", as = f32, ui(primary, min = 0.0, max = 32.0, format = "%.2f", tooltip = "Number of branches leaving the main channel; the fraction fades the last one", group = "branch"))]
    #[persist(owner = Frame, name = branch_zone_start, path = "branch.zone_start", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Where along the main channel (0 = start, 1 = end) branches begin", group = "branch"))]
    #[persist(owner = Frame, name = branch_zone_end, path = "branch.zone_end", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Where along the main channel (0 = start, 1 = end) branches stop", group = "branch"))]
    #[persist(owner = Frame, name = branch_angle, path = "branch.angle", as = f32, ui(primary, min = 0.0, max = 1.57, format = "%.2f", tooltip = "Radians between a branch and its parent channel", group = "branch"))]
    #[persist(owner = Frame, name = branch_length_ratio, path = "branch.length_ratio", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", group = "branch"))]
    #[persist(owner = Frame, name = branch_radius_ratio, path = "branch.radius_ratio", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", group = "branch"))]
    #[persist(owner = Frame, name = branch_intensity_ratio, path = "branch.intensity_ratio", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", group = "branch"))]
    pub branch: LightningBranching,
    #[persist(owner = Frame, name = core_intensity, path = "look.core_intensity", as = f32, ui(primary, min = 0.0, max = 200.0, format = "%.1f", tooltip = "Radiance of the saturated channel core", group = "look"))]
    #[persist(owner = Frame, name = core_color, path = "look.core_color", as = [f32; 3], scalars = rgb, ui(kind = Color, min = 0.0, max = 1.0, format = "%.2f", group = "look"))]
    #[persist(owner = Frame, name = rim_ratio, path = "look.rim_ratio", as = f32, ui(min = 1.0, max = 20.0, format = "%.2f", tooltip = "Radius of the rim sheath as a multiple of the core radius", group = "look"))]
    #[persist(owner = Frame, name = rim_intensity, path = "look.rim_intensity", as = f32, ui(min = 0.0, max = 100.0, format = "%.2f", group = "look"))]
    #[persist(owner = Frame, name = rim_color, path = "look.rim_color", as = [f32; 3], scalars = rgb, ui(primary, kind = Color, min = 0.0, max = 1.0, format = "%.2f", group = "look"))]
    #[persist(owner = Frame, name = beam_radius, path = "look.beam_radius", as = f32, ui(min = 0.0, max = 10.0, format = "%.2f", tooltip = "Radius of the beam the arcs wrap around; 0 keeps a bare channel", group = "look"))]
    #[persist(owner = Frame, name = beam_arc_count, path = "look.beam_arc_count", as = u32, ui(min = 0.0, max = 64.0, format = "%.0f", group = "look"))]
    #[persist(owner = Frame, name = flash_gain, path = "look.flash_gain", as = f32, ui(min = 0.0, max = 10.0, format = "%.2f", tooltip = "Radiance of the ambient flash emitted while a stroke is alive; 0 disables it", group = "look"))]
    #[persist(owner = Frame, name = flash_radius, path = "look.flash_radius", as = f32, ui(min = 0.0, max = 100.0, format = "%.2f", group = "look"))]
    pub look: LightningLook,
    #[persist(owner = Frame, name = end_variance, path = "shape.end_variance", as = f32, ui(min = 0.0, max = 4.0, format = "%.2f", tooltip = "Radius each discharge scatters its end point by, across the bolt's direction", group = "shape"))]
    #[persist(owner = Frame, name = growth_time, path = "timing.growth_time", as = f32, ui(min = 0.0, max = 1.0, format = "%.3f", tooltip = "Seconds the discharge takes to reach its end; 0 draws it whole at once", group = "timing"))]
    #[persist(owner = Frame, name = burst_start, path = "timing.burst_start", as = f32, ui(min = 0.0, max = 10.0, format = "%.2f", group = "timing"))]
    #[persist(owner = Frame, name = burst_interval, path = "timing.burst_interval", as = f32, ui(primary, min = 0.01, max = 10.0, format = "%.2f", group = "timing"))]
    #[persist(owner = Frame, name = burst_jitter, path = "timing.burst_jitter", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Fraction of burst_interval the burst start is randomly shifted by", group = "timing"))]
    #[persist(owner = Frame, name = burst_count, path = "timing.burst_count", as = u32, ui(min = 0.0, max = 16.0, format = "%.0f", tooltip = "Number of bursts to play; 0 repeats forever", group = "timing"))]
    #[persist(owner = Frame, name = attack_time, path = "timing.attack_time", as = f32, ui(min = 0.0, max = 1.0, format = "%.3f", group = "timing"))]
    #[persist(owner = Frame, name = sustain_time, path = "timing.sustain_time", as = f32, ui(min = 0.0, max = 1.0, format = "%.3f", group = "timing"))]
    #[persist(owner = Frame, name = release_time, path = "timing.release_time", as = f32, ui(min = 0.0, max = 2.0, format = "%.3f", group = "timing"))]
    #[persist(owner = Frame, name = stroke_count, path = "timing.stroke_count", as = u32, ui(min = 1.0, max = 8.0, format = "%.0f", tooltip = "Return strokes per burst, played stroke_interval apart", group = "timing"))]
    #[persist(owner = Frame, name = stroke_interval, path = "timing.stroke_interval", as = f32, ui(min = 0.001, max = 1.0, format = "%.3f", group = "timing"))]
    #[persist(owner = Frame, name = stroke_decay, path = "timing.stroke_decay", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Intensity of each return stroke relative to the previous one", group = "timing"))]
    #[persist(owner = Frame, name = flicker_amplitude, path = "timing.flicker_amplitude", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", group = "timing"))]
    #[persist(owner = Frame, name = flicker_period, path = "timing.flicker_period", as = f32, ui(min = 0.001, max = 1.0, format = "%.3f", group = "timing"))]
    #[persist(owner = Frame, name = reseed_level, path = "timing.reseed_level", as = u32, ui(min = 0.0, max = 8.0, format = "%.0f", tooltip = "Subdivision level above which the channel is reshaped every reseed_period", group = "timing"))]
    #[persist(owner = Frame, name = reseed_period, path = "timing.reseed_period", as = f32, ui(min = 0.01, max = 10.0, format = "%.2f", group = "timing"))]
    #[persist(owner = Frame, name = charge_ramp, path = "timing.charge_ramp", as = f32, ui(min = 0.0, max = 2.0, format = "%.2f", tooltip = "Seconds of dim leader glow before the first return stroke; 0 strikes instantly", group = "timing"))]
    #[persist(owner = Frame, name = seed, path = "timing.seed", as = u32, ui(min = 0.0, max = 9999.0, format = "%.0f", tooltip = "Seed of the deterministic hash that shapes the channel and the burst jitter", group = "timing"))]
    pub timing: LightningTiming,
}

impl Default for LightningEffect {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            shape: LightningShape::default(),
            branch: LightningBranching::default(),
            look: LightningLook::default(),
            timing: LightningTiming::default(),
        }
    }
}

pub fn build_lightning_model_matrix(effect: &LightningEffect) -> Matrix4<f32> {
    Matrix4::from_translation(effect.position) * Matrix4::from(effect.rotation)
}
