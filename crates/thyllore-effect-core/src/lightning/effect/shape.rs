use crate::lightning::LightningParameterOwner;
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SnapshotValues;

pub const LIGHTNING_MAX_WAYPOINTS: usize = 8;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
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
#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = LightningParameterOwner, owner = Frame, group = "shape")]
pub struct LightningShape {
    /// End point of the main strike relative to the effect origin
    #[persist(code = [768, 769, 770], debug_range = (-20.0, 20.0), ui(kind = Offset, min = -50.0, max = 50.0))]
    pub end_offset: [f32; 3],
    #[persist]
    pub source: LightningSource,
    #[persist(code = 771, debug_range = (1.0, 8.0), ui(min = 1.0, max = 32.0, format = "%.0f"))]
    pub strikes_per_burst: u32,
    /// Midpoint displacement subdivisions of the channel; the segment count is 2^detail_levels
    #[persist(code = 772, ui(min = 1.0, max = 8.0, format = "%.0f"))]
    pub detail_levels: u32,
    /// Lateral displacement of the first subdivision relative to the strike length
    #[persist(code = 773, ui(primary, min = 0.0, max = 2.0))]
    pub tortuosity: f32,
    /// Decay of the displacement per subdivision level; 0.5 is a Brownian channel
    #[persist(code = 774, ui(min = 0.0, max = 1.0))]
    pub roughness: f32,
    #[persist(code = 775, ui(primary, min = 0.001, max = 1.0, format = "%.3f"))]
    pub core_radius: f32,
    /// Core radius at the strike tip as a fraction of core_radius
    #[persist(code = 776, ui(min = 0.0, max = 1.0))]
    pub tip_radius_ratio: f32,
    /// Fraction of the core radius over which the coverage falls to zero
    #[persist(code = 777, ui(min = 0.0, max = 1.0))]
    pub edge_fraction: f32,
    /// Radius each discharge scatters its end point by, across the bolt's direction
    #[persist(code = 778, ui(min = 0.0, max = 4.0))]
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
