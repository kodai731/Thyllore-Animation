use crate::ecs::systems::world::{run_batch_schedule_phase, run_frame_clock_phase};

pub fn run_first_phase(world: &crate::ecs::World) {
    run_frame_clock_phase(world);
    run_batch_schedule_phase(world);
}
