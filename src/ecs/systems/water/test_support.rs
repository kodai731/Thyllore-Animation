use super::spawn_water;
use crate::ecs::component::WaterTorusEffect;
use crate::ecs::systems::effect_time::{EffectTimeSources, TimelineSample};
use crate::ecs::world::{Entity, World};

pub(super) fn spawn_default_water(world: &mut World, name: &str) -> Entity {
    spawn_water(world, name, WaterTorusEffect::default())
}

pub(super) fn timeline_time_sources(
    current_time: f32,
    playing: bool,
    delta_time: f32,
) -> EffectTimeSources {
    EffectTimeSources {
        batch_fixed_time: None,
        fixed_step_time: None,
        timeline: Some(TimelineSample {
            current_time,
            playing,
        }),
        delta_time,
        free_run_when_paused: true,
    }
}
