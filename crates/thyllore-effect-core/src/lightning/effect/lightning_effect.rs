use cgmath::{Matrix4, Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SnapshotValues;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum LightningSource {
    #[default]
    Point,
    Shell {
        radius: f32,
    },
}

impl SnapshotValues for LightningSource {
    fn snapshot_values(&self) -> Vec<f32> {
        match self {
            LightningSource::Point => vec![0.0],
            LightningSource::Shell { radius } => vec![1.0, *radius],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightningEffect {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub time: f32,
    pub time_scale: f32,
    pub time_offset: f32,
    pub end_offset: [f32; 3],
    pub source: LightningSource,
    pub seed: u32,
    pub strikes_per_burst: u32,
    pub detail_levels: u32,
    pub tortuosity: f32,
    pub roughness: f32,
    pub core_radius: f32,
    pub tip_radius_ratio: f32,
    pub edge_fraction: f32,
    pub branch_depth: u32,
    pub branch_probability: f32,
    pub branch_angle: f32,
    pub branch_length_ratio: f32,
    pub branch_radius_ratio: f32,
    pub branch_intensity_ratio: f32,
    pub core_intensity: f32,
    pub core_color: [f32; 3],
    pub glow_ratio: f32,
    pub glow_intensity: f32,
    pub glow_color: [f32; 3],
    pub burst_start: f32,
    pub burst_interval: f32,
    pub burst_jitter: f32,
    pub burst_count: u32,
    pub attack_time: f32,
    pub sustain_time: f32,
    pub release_time: f32,
    pub stroke_count: u32,
    pub stroke_interval: f32,
    pub stroke_decay: f32,
    pub flicker_amplitude: f32,
    pub flicker_period: f32,
    pub reseed_level: u32,
    pub reseed_period: f32,
    pub charge_ramp: f32,
    pub beam_radius: f32,
    pub beam_arc_count: u32,
    pub flash_gain: f32,
    pub flash_radius: f32,
}

impl Default for LightningEffect {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            end_offset: [0.0, -8.0, 0.0],
            source: LightningSource::Point,
            seed: 0,
            strikes_per_burst: 1,
            detail_levels: 5,
            tortuosity: 0.35,
            roughness: 0.6,
            core_radius: 0.05,
            tip_radius_ratio: 0.3,
            edge_fraction: 0.5,
            branch_depth: 2,
            branch_probability: 0.3,
            branch_angle: 0.6,
            branch_length_ratio: 0.5,
            branch_radius_ratio: 0.6,
            branch_intensity_ratio: 0.5,
            core_intensity: 40.0,
            core_color: [1.0, 1.0, 1.0],
            glow_ratio: 4.0,
            glow_intensity: 6.0,
            glow_color: [0.45, 0.65, 1.0],
            burst_start: 0.0,
            burst_interval: 2.0,
            burst_jitter: 0.2,
            burst_count: 1,
            attack_time: 0.01,
            sustain_time: 0.04,
            release_time: 0.12,
            stroke_count: 1,
            stroke_interval: 0.05,
            stroke_decay: 0.6,
            flicker_amplitude: 0.25,
            flicker_period: 0.03,
            reseed_level: 2,
            reseed_period: 1.0,
            charge_ramp: 0.0,
            beam_radius: 0.0,
            beam_arc_count: 8,
            flash_gain: 0.0,
            flash_radius: 2.0,
        }
    }
}

pub fn build_lightning_model_matrix(effect: &LightningEffect) -> Matrix4<f32> {
    Matrix4::from_translation(effect.position) * Matrix4::from(effect.rotation)
}
