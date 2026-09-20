use crate::ecs::component::{FlameBaked, FlameEffect, FlameTemporalAccum};
use crate::ecs::resource::{
    FlameHistorySnapshot, FlameHistorySnapshotState, FlameRenderSettings, FrameClock,
    ProjectionData,
};
use crate::ecs::world::World;

const STABLE_FRAME_HISTORY_WEIGHT: f32 = 0.85;

/// Reusing the previous frame's shading is only valid while the camera and the flame
/// parameters hold still. A fixed-step run never reuses history so a single-frame screenshot
/// stays deterministic.
pub fn accumulate_flame_history(world: &mut World) {
    let flame_entities = world.entities_with::<FlameEffect>();
    let count = flame_entities.len();

    if count == 0 {
        return;
    }

    // If there are 2 or more instances, only increment the frame counter for all
    if count >= 2 {
        for &entity in &flame_entities {
            let next = world
                .get_component::<FlameTemporalAccum>(entity)
                .map(|t| t.frame_index.wrapping_add(1))
                .unwrap_or(0);
            world.insert_component(
                entity,
                FlameTemporalAccum {
                    weight: 0.0,
                    frame_index: next,
                },
            );
        }
        return;
    }

    // Single instance: perform original snapshot comparison logic
    let entity = flame_entities[0];
    // Collect data first to avoid borrow conflicts
    let view = world.resource::<ProjectionData>().view;
    let settings = *world.resource::<FlameRenderSettings>();
    let fixed_step_frame = fixed_step_frame(world);
    let old_effect = world.get_component::<FlameEffect>(entity).cloned();
    let Some(old_effect) = old_effect else {
        return;
    };
    let baked = world
        .get_component::<FlameBaked>(entity)
        .cloned()
        .unwrap_or_default();
    let old_temporal = world
        .get_component::<FlameTemporalAccum>(entity)
        .cloned()
        .unwrap_or_default();

    let snapshot = FlameHistorySnapshot {
        view,
        appearance: strip_per_frame_state(&old_effect),
        baked,
        settings,
    };

    // Get matches_previous_frame before taking mutable borrow
    let state = world.resource::<FlameHistorySnapshotState>();
    let matches_previous_frame = state.previous.as_ref() == Some(&snapshot);
    drop(state);

    // Update the temporal state
    let mut state = world.resource_mut::<FlameHistorySnapshotState>();
    state.previous = Some(snapshot);
    drop(state);

    world.insert_component(
        entity,
        FlameTemporalAccum {
            frame_index: fixed_step_frame.unwrap_or(old_temporal.frame_index.wrapping_add(1)),
            weight: if matches_previous_frame && fixed_step_frame.is_none() {
                STABLE_FRAME_HISTORY_WEIGHT
            } else {
                0.0
            },
        },
    );
}

/// The clock advances on its own every frame and must be excluded, otherwise
/// the snapshot never compares equal and history would be discarded
/// unconditionally.
fn strip_per_frame_state(effect: &FlameEffect) -> FlameEffect {
    let mut appearance = effect.clone();
    appearance.time = 0.0;
    appearance
}

fn fixed_step_frame(world: &World) -> Option<u64> {
    let clock = world.get_resource::<FrameClock>()?;
    clock.is_fixed().then_some(clock.frame)
}
