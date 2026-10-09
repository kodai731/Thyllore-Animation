use crate::ecs::component::{WaterTemporalAccum, WaterTorusEffect};
use crate::ecs::resource::{ProjectionData, WaterHistorySnapshot, WaterRenderSettings};
use crate::ecs::systems::temporal_history::{
    accumulate_temporal_history, HistoryFrame, HistoryReuse, TemporalHistory,
};
use crate::ecs::world::{Entity, World};
use crate::ecs::FrameContext;

const STABLE_FRAME_HISTORY_WEIGHT: f32 = 0.85;

/// A fixed-step run keeps reusing history so a batch capture converges like the live view.
pub struct WaterHistory;

impl TemporalHistory for WaterHistory {
    type Effect = WaterTorusEffect;
    type Snapshot = WaterHistorySnapshot;
    type Accum = WaterTemporalAccum;

    fn snapshot(world: &World, entity: Entity) -> Option<WaterHistorySnapshot> {
        let effect = world.get_component::<WaterTorusEffect>(entity)?;
        Some(WaterHistorySnapshot {
            view: world.resource::<ProjectionData>().view,
            effect: strip_per_frame_state(&effect),
            settings: *world.resource::<WaterRenderSettings>(),
        })
    }

    fn frame_index(accum: &WaterTemporalAccum) -> u64 {
        accum.frame_index
    }

    fn accum(world: &World, frame: HistoryFrame) -> WaterTemporalAccum {
        let settings = *world.resource::<WaterRenderSettings>();
        let (weight, history_invalidated) = match frame.reuse {
            HistoryReuse::Continue => (
                settings
                    .batch_history_weight
                    .unwrap_or(STABLE_FRAME_HISTORY_WEIGHT),
                false,
            ),
            HistoryReuse::Restart => (0.0, true),
            HistoryReuse::Unchecked => (0.0, false),
        };
        WaterTemporalAccum {
            weight,
            frame_index: frame.frame_index,
            history_invalidated,
        }
    }
}

pub fn accumulate_water_history(world: &mut World) {
    accumulate_temporal_history::<WaterHistory>(world);
}

/// The clock advances on its own every frame and must be excluded, otherwise
/// the snapshot never compares equal and history would be discarded
/// unconditionally.
fn strip_per_frame_state(effect: &WaterTorusEffect) -> WaterTorusEffect {
    let mut appearance = effect.clone();
    appearance.time = 0.0;
    appearance
}

crate::frame_prep_hook!("water", Accumulate, water_accumulate);

fn water_accumulate(ctx: &mut FrameContext) {
    accumulate_water_history(&mut ctx.world);
}
