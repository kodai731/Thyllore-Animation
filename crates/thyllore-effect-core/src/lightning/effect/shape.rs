use serde::{Deserialize, Serialize};
use thyllore_scene_core::SnapshotValues;

pub const LIGHTNING_MAX_WAYPOINTS: usize = 8;

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

/// Geometry of the main channel: where it starts and ends, and how it is subdivided.
#[derive(Clone, Debug, PartialEq)]
pub struct LightningShape {
    pub end_offset: [f32; 3],
    pub source: LightningSource,
    pub strikes_per_burst: u32,
    pub detail_levels: u32,
    pub tortuosity: f32,
    pub roughness: f32,
    pub core_radius: f32,
    pub tip_radius_ratio: f32,
    pub edge_fraction: f32,
    pub end_variance: f32,
    pub waypoints: [[f32; 3]; LIGHTNING_MAX_WAYPOINTS],
    pub waypoint_count: u32,
}

impl Default for LightningShape {
    fn default() -> Self {
        Self {
            end_offset: [0.0, -8.0, 0.0],
            source: LightningSource::Point,
            strikes_per_burst: 1,
            detail_levels: 5,
            tortuosity: 0.35,
            roughness: 0.6,
            core_radius: 0.05,
            tip_radius_ratio: 0.3,
            edge_fraction: 0.3,
            end_variance: 0.0,
            waypoints: [[0.0; 3]; LIGHTNING_MAX_WAYPOINTS],
            waypoint_count: 0,
        }
    }
}
