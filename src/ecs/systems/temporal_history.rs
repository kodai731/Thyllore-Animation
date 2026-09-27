use crate::ecs::resource::{FrameClock, HistorySnapshotState};
use crate::ecs::storage::Component;
use crate::ecs::world::{Entity, World};

/// Whether this frame may reuse the shading accumulated by the previous one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryReuse {
    /// The snapshot equals the previous frame's.
    Continue,
    /// The snapshot changed, or there was no previous frame.
    Restart,
    /// Several instances share one history, so no snapshot is compared.
    Unchecked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStepping {
    WallClock,
    Fixed,
}

/// What an effect's accumulator is built from for one frame.
#[derive(Clone, Copy, Debug)]
pub struct HistoryFrame {
    pub frame_index: u64,
    pub reuse: HistoryReuse,
    pub stepping: FrameStepping,
}

/// An effect whose shading is accumulated over frames while its snapshot holds still.
pub trait TemporalHistory {
    type Effect: Component;
    type Snapshot: PartialEq + 'static;
    type Accum: Component + Clone + Default;

    fn snapshot(world: &World, entity: Entity) -> Option<Self::Snapshot>;
    fn frame_index(accum: &Self::Accum) -> u64;
    fn accum(world: &World, frame: HistoryFrame) -> Self::Accum;
}

/// Reusing the previous frame's shading is only valid while the camera and the effect
/// parameters hold still; the effect decides what a fixed-step run does with a match.
pub fn accumulate_temporal_history<H: TemporalHistory>(world: &mut World) {
    let entities = world.entities_with::<H::Effect>();
    if entities.is_empty() {
        return;
    }
    let stepping = frame_stepping(world);
    let fixed_step_frame = fixed_step_frame(world);

    if entities.len() >= 2 {
        for &entity in &entities {
            let frame_index = world
                .get_component::<H::Accum>(entity)
                .map(|accum| H::frame_index(&accum).wrapping_add(1))
                .unwrap_or(0);
            let frame = HistoryFrame {
                frame_index,
                reuse: HistoryReuse::Unchecked,
                stepping,
            };
            world.insert_component(entity, H::accum(world, frame));
        }
        return;
    }

    let entity = entities[0];
    let Some(snapshot) = H::snapshot(world, entity) else {
        return;
    };
    let previous_frame_index = world
        .get_component::<H::Accum>(entity)
        .map(|accum| H::frame_index(&accum))
        .unwrap_or_default();

    let mut state = world.resource_mut::<HistorySnapshotState<H::Snapshot>>();
    let reuse = if state.previous.as_ref() == Some(&snapshot) {
        HistoryReuse::Continue
    } else {
        HistoryReuse::Restart
    };
    state.previous = Some(snapshot);
    drop(state);

    let frame = HistoryFrame {
        frame_index: fixed_step_frame.unwrap_or(previous_frame_index.wrapping_add(1)),
        reuse,
        stepping,
    };
    world.insert_component(entity, H::accum(world, frame));
}

fn frame_stepping(world: &World) -> FrameStepping {
    match fixed_step_frame(world) {
        Some(_) => FrameStepping::Fixed,
        None => FrameStepping::WallClock,
    }
}

fn fixed_step_frame(world: &World) -> Option<u64> {
    let clock = world.get_resource::<FrameClock>()?;
    clock.is_fixed().then_some(clock.frame)
}
