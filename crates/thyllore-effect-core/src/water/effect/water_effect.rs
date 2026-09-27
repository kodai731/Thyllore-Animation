use cgmath::{Matrix4, Quaternion, Vector3};

use crate::water::ownership::WaterParameterOwner;

#[derive(Clone, Debug, PartialEq, crate::UboPack, thyllore_scene_core::SceneFields)]
#[ubo(target = crate::WaterUBO)]
#[scene(key = "water_torus", tag = WaterParameterOwner, owner = Frame, tags = WATER_PARAMETER_OWNERSHIP, snapshot = water_parameter_snapshot, scalars = WATER_SCALAR_PARAMS, ui = WATER_UI_PARAMS, overwrite = overwrite_water_persisted_fields)]
pub struct WaterTorusEffect {
    #[persist(as = [f32; 3])]
    pub position: Vector3<f32>,
    #[persist(as = [f32; 4], with = crate::scene_convert::quaternion_wxyz)]
    pub rotation: Quaternion<f32>,
    #[ubo("radii.x")]
    #[persist(ui(min = 0.01, max = 10.0, group = "shape"))]
    pub major_radius: f32,
    #[ubo("radii.y")]
    #[persist(ui(min = 0.01, max = 5.0, group = "shape"))]
    pub minor_radius: f32,
    #[ubo("absorption.w")]
    #[persist(ui(min = 1.0, max = 2.5, format = "%.3f", group = "optics"))]
    pub ior: f32,
    #[ubo("absorption.xyz")]
    /// Beer-Lambert absorption per meter; the picker shows the colour transmitted over the reference distance
    #[persist(ui(kind = Absorption, min = 0.0, max = 10.0, group = "optics"))]
    pub absorption: [f32; 3],
    #[ubo("flow.x")]
    #[persist(ui(primary, min = -5.0, max = 5.0, group = "flow"))]
    pub flow_longitudinal: f32,
    #[ubo("flow.y")]
    #[persist(ui(min = -5.0, max = 5.0, group = "flow"))]
    pub flow_meridional: f32,
    #[persist(ui(primary, min = 0.0, max = 1.0, format = "%.3f", group = "wave"))]
    pub wave_amplitude: f32,
    #[persist(ui(primary, min = 0.0, max = 50.0, format = "%.1f", group = "wave"))]
    pub wave_frequency: f32,
    #[persist(ui(min = 0.0, max = 10.0, group = "wave"))]
    pub wave_speed: f32,
    #[persist(ui(min = 0.0, max = 1.0, group = "wave"))]
    pub wave_dispersion: f32,
    #[persist(ui(min = 0.0, max = 1.0, group = "wave"))]
    pub wave_lb_blend: f32,
    #[ubo("lighting.x")]
    #[persist(ui(primary, min = 0.0, max = 20.0, group = "lighting"))]
    pub light_intensity: f32,
    #[ubo("lighting.y")]
    #[persist(ui(min = 1.0, max = 1024.0, format = "%.0f", group = "lighting"))]
    pub highlight_sharpness: f32,
    #[ubo("lighting.z")]
    #[persist(ui(min = 0.0, max = 2.0, group = "lighting"))]
    pub sky_brightness: f32,
    #[ubo("lighting.w")]
    #[persist(ui(min = 0.0, max = 10.0, group = "lighting"))]
    pub scatter_strength: f32,
    #[ubo("scattering.x")]
    #[persist(ui(min = -0.9, max = 0.9, group = "lighting"))]
    pub scatter_anisotropy: f32,
    #[ubo("composite.x")]
    #[persist(ui(primary, min = 0.0, max = 1.0, group = "look"))]
    pub reflect_strength: f32,
    #[ubo("composite.y")]
    #[persist(ui(min = 0.0, max = 1.0, group = "look"))]
    pub refract_strength: f32,
    #[ubo("radii.z")]
    #[persist(ui(min = 0.0, max = 2.0, group = "look"))]
    pub caustic_strength: f32,
    #[ubo("tint.xyz")]
    /// Scattering tint
    #[persist(ui(primary, kind = Color, min = 0.0, max = 1.0, group = "look"))]
    pub tint: [f32; 3],
    #[ubo("flow.z")]
    #[runtime(ui(min = 0.0, max = 100.0))]
    pub time: f32,
    #[runtime(ui(primary, min = 0.0, max = 4.0))]
    pub time_scale: f32,
    #[runtime(ui(min = -100.0, max = 100.0))]
    pub time_offset: f32,
}

impl Default for WaterTorusEffect {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            major_radius: 1.0,
            minor_radius: 0.3,
            ior: 1.333,
            absorption: [0.35, 0.08, 0.02],
            flow_longitudinal: 0.2,
            flow_meridional: 0.0,
            wave_amplitude: 0.02,
            wave_frequency: 6.0,
            wave_speed: 1.0,
            wave_dispersion: 0.0,
            wave_lb_blend: 0.0,
            reflect_strength: 1.0,
            refract_strength: 1.0,
            caustic_strength: 0.6,
            light_intensity: 1.0,
            highlight_sharpness: 64.0,
            sky_brightness: 1.0,
            scatter_strength: 1.0,
            scatter_anisotropy: 0.0,
            tint: [0.05, 0.25, 0.35],
        }
    }
}

pub fn build_water_model_matrix(effect: &WaterTorusEffect) -> Matrix4<f32> {
    Matrix4::from_translation(effect.position) * Matrix4::from(effect.rotation)
}

pub fn advance_water_time(effect: &mut WaterTorusEffect, delta_time: f32) {
    effect.time += delta_time.max(0.0);
}

impl crate::EffectPresets for WaterTorusEffect {
    const PRESET_NAMES: &'static [&'static str] = crate::WATER_PRESET_NAMES;

    fn apply_preset(&mut self, name: &str) -> bool {
        crate::apply_water_preset(self, name)
    }
}

impl crate::Placement for WaterTorusEffect {
    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
        self.time = time;
        self.position = Vector3::from(position);
        crate::scene_convert::quaternion_wxyz::set(&mut self.rotation, rotation);
    }
}
