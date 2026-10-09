use crate::ecs::component::{FlameBaked, FlameEffect, FlameTemporalAccum};
use crate::ecs::resource::{FlameHistorySnapshot, FlameRenderSettings, ProjectionData};
use crate::ecs::systems::temporal_history::{
    accumulate_temporal_history, FrameStepping, HistoryFrame, HistoryReuse, TemporalHistory,
};
use crate::ecs::world::{Entity, World};

const STABLE_FRAME_HISTORY_WEIGHT: f32 = 0.85;

/// A fixed-step run never reuses history so a single-frame screenshot stays deterministic.
pub struct FlameHistory;

impl TemporalHistory for FlameHistory {
    type Effect = FlameEffect;
    type Snapshot = FlameHistorySnapshot;
    type Accum = FlameTemporalAccum;

    fn snapshot(world: &World, entity: Entity) -> Option<FlameHistorySnapshot> {
        let effect = world.get_component::<FlameEffect>(entity)?;
        Some(FlameHistorySnapshot {
            view: world.resource::<ProjectionData>().view,
            appearance: strip_per_frame_state(&effect),
            baked: world
                .get_component::<FlameBaked>(entity)
                .cloned()
                .unwrap_or_default(),
            settings: *world.resource::<FlameRenderSettings>(),
        })
    }

    fn frame_index(accum: &FlameTemporalAccum) -> u64 {
        accum.frame_index
    }

    fn accum(_: &World, frame: HistoryFrame) -> FlameTemporalAccum {
        let weight = match (frame.reuse, frame.stepping) {
            (HistoryReuse::Continue, FrameStepping::WallClock) => STABLE_FRAME_HISTORY_WEIGHT,
            (HistoryReuse::Continue, FrameStepping::Fixed)
            | (HistoryReuse::Restart, _)
            | (HistoryReuse::Unchecked, _) => 0.0,
        };
        FlameTemporalAccum {
            weight,
            frame_index: frame.frame_index,
        }
    }
}

pub fn accumulate_flame_history(world: &mut World) {
    accumulate_temporal_history::<FlameHistory>(world);
}

/// The clock advances on its own every frame and must be excluded, otherwise
/// the snapshot never compares equal and history would be discarded
/// unconditionally.
fn strip_per_frame_state(effect: &FlameEffect) -> FlameEffect {
    let mut appearance = effect.clone();
    appearance.time = 0.0;
    appearance
}
