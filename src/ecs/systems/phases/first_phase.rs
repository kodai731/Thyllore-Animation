use crate::ecs::systems::world::{advance_frame_clock, batch_run_request_scheduled_capture};

pub fn run_first_phase(world: &crate::ecs::World) {
    advance_frame_clock(world);
    batch_run_request_scheduled_capture(world);
}
