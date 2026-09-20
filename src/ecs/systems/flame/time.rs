use crate::ecs::component::{
    apply_flame_param_value, FlameBaked, FlameEffect, FlameParam, FlameTemporalAccum,
};
use crate::ecs::resource::LightState;
use crate::ecs::systems::effect_time::{advance_effect_time, EffectTimeSources, TimedEffect};
use crate::ecs::world::{Entity, Transform, World};
use crate::ecs::FrameContext;
use cgmath::Vector3;
use thyllore_anim_core::editable::PropertyType;

impl TimedEffect for FlameEffect {
    type WorldInputs = Option<Vector3<f32>>;

    fn entities(world: &World) -> Vec<Entity> {
        world.entities_with::<FlameEffect>()
    }

    fn time_sources(world: &World, delta_time: f32) -> EffectTimeSources {
        EffectTimeSources::collect(world, delta_time, None, false)
    }

    fn collect_world_inputs(world: &World) -> Option<Vector3<f32>> {
        world
            .get_resource::<LightState>()
            .map(|light| light.light_position)
    }

    fn time(&self) -> f32 {
        self.time
    }

    fn time_mut(&mut self) -> &mut f32 {
        &mut self.time
    }

    fn time_scale(&self) -> f32 {
        self.time_scale
    }

    fn time_offset(&self) -> f32 {
        self.time_offset
    }

    fn apply_world_inputs(&mut self, light_position: Option<Vector3<f32>>) {
        if let Some(light_position) = light_position {
            self.light_position_world = light_position;
        }
    }

    fn place(&mut self, transform: &Transform) {
        self.position = transform.translation;
        self.rotation = transform.rotation;
    }

    fn apply_scalar(&mut self, property_type: PropertyType, value: f32) {
        if let Some(param) = FlameParam::from_property_type(property_type) {
            apply_flame_param_value(self, param, value);
        }
    }
}

pub fn flame_time_advance(ctx: &mut FrameContext) {
    ensure_flame_runtime_components(ctx.world);
    advance_effect_time::<FlameEffect>(ctx);
}

/// Baked data and the temporal accumulator are runtime state every flame carries, whichever
/// path spawned it.
pub fn ensure_flame_runtime_components(world: &mut World) {
    for entity in world.entities_with::<FlameEffect>() {
        if !world.has_component::<FlameBaked>(entity) {
            world.insert_component(entity, FlameBaked::default());
        }
        if !world.has_component::<FlameTemporalAccum>(entity) {
            world.insert_component(entity, FlameTemporalAccum::default());
        }
    }
}
