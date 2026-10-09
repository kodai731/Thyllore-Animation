use crate::ecs::systems::batch_apply_scheduled_actions;
use crate::ecs::systems::world::{advance_frame_clock, batch_run_request_scheduled_capture};

pub fn run_first_phase(world: &mut crate::ecs::World) {
    advance_frame_clock(world);
    batch_run_request_scheduled_capture(world);
    batch_apply_scheduled_actions(world);
}
