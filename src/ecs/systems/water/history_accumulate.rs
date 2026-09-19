use crate::ecs::component::{WaterTemporalAccum, WaterTorusEffect};
use crate::ecs::resource::{
    FrameClock, ProjectionData, WaterHistorySnapshot, WaterHistorySnapshotState,
    WaterRenderSettings,
};
use crate::ecs::world::World;

const STABLE_FRAME_HISTORY_WEIGHT: f32 = 0.85;

/// Reusing the previous frame's shading is only valid while the camera and the water
/// parameters hold still. A fixed-step run never reuses history so a single-frame screenshot
/// stays deterministic.
pub fn accumulate_water_history(world: &mut World) {
    let water_entities = world.query_waters();
    let count = water_entities.len();

    if count == 0 {
        return;
    }

    // If there are 2 or more instances, only increment the frame counter for all
    if count >= 2 {
        for &entity in &water_entities {
            let next = world
                .get_component::<WaterTemporalAccum>(entity)
                .map(|t| t.frame_index.wrapping_add(1))
                .unwrap_or(0);
            world.insert_component(
                entity,
                WaterTemporalAccum {
                    weight: 0.0,
                    frame_index: next,
                    history_invalidated: false,
                },
            );
        }
        return;
    }

    // Single instance: perform original snapshot comparison logic
    let entity = water_entities[0];
    // Collect data first to avoid borrow conflicts
    let view = world.resource::<ProjectionData>().view;
    let settings = *world.resource::<WaterRenderSettings>();
    let fixed_step_frame = fixed_step_frame(world);
    let old_effect = world.get_component::<WaterTorusEffect>(entity).cloned();
    let Some(old_effect) = old_effect else {
        return;
    };
    let old_temporal = world
        .get_component::<WaterTemporalAccum>(entity)
        .cloned()
        .unwrap_or_default();

    let snapshot = WaterHistorySnapshot {
        view,
        effect: strip_per_frame_state(&old_effect),
        settings,
    };

    // Get matches_previous_frame before taking mutable borrow
    let state = world.resource::<WaterHistorySnapshotState>();
    let matches_previous_frame = state.previous.as_ref() == Some(&snapshot);
    drop(state);

    // Update the temporal state
    let mut state = world.resource_mut::<WaterHistorySnapshotState>();
    state.previous = Some(snapshot);
    drop(state);

    world.insert_component(
        entity,
        WaterTemporalAccum {
            frame_index: fixed_step_frame.unwrap_or(old_temporal.frame_index.wrapping_add(1)),
            weight: if matches_previous_frame {
                settings
                    .batch_history_weight
                    .unwrap_or(STABLE_FRAME_HISTORY_WEIGHT)
            } else {
                0.0
            },
            history_invalidated: !matches_previous_frame,
        },
    );
}

/// The clock advances on its own every frame and must be excluded, otherwise
/// the snapshot never compares equal and history would be discarded
/// unconditionally.
fn strip_per_frame_state(effect: &WaterTorusEffect) -> WaterTorusEffect {
    let mut appearance = effect.clone();
    appearance.time = 0.0;
    appearance
}

fn fixed_step_frame(world: &World) -> Option<u64> {
    let clock = world.get_resource::<FrameClock>()?;
    clock.is_fixed().then_some(clock.frame)
}
