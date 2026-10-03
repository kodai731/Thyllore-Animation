use cgmath::Matrix4;

use super::FlameRenderSettings;
use crate::ecs::component::{FlameBaked, FlameEffect};
use crate::ecs::resource::HistorySnapshotState;

/// The frame state that history reuse depends on. Any difference between two
/// consecutive frames invalidates the accumulated history.
#[derive(Clone, PartialEq)]
pub struct FlameHistorySnapshot {
    pub view: Matrix4<f32>,
    pub appearance: FlameEffect,
    pub baked: FlameBaked,
    pub settings: FlameRenderSettings,
}

pub type FlameHistorySnapshotState = HistorySnapshotState<FlameHistorySnapshot>;

crate::startup_resource!(FlameHistorySnapshotState, PostProcessing);
