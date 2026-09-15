use crate::ecs::component::{apply_lightning_param_value, LightningEffect, LightningParam};
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::effect_time::{advance_effect_time, EffectTimeSources, TimedEffect};
use crate::ecs::world::{Entity, Transform, World};
use crate::ecs::FrameContext;
use thyllore_anim_core::editable::PropertyType;

impl TimedEffect for LightningEffect {
    type WorldInputs = ();

    fn entities(world: &World) -> Vec<Entity> {
        world.query_lightnings()
    }

    fn time_sources(world: &World, delta_time: f32) -> EffectTimeSources {
        let batch_fixed_time = world
            .get_resource::<LightningRenderSettings>()
            .and_then(|settings| settings.batch_fixed_time);
        EffectTimeSources::collect(world, delta_time, batch_fixed_time, false)
    }

    fn collect_world_inputs(_world: &World) {}

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

    fn apply_world_inputs(&mut self, _inputs: ()) {}

    fn place(&mut self, transform: &Transform) {
        self.position = transform.translation;
        self.rotation = transform.rotation;
    }

    fn apply_scalar(&mut self, property_type: PropertyType, value: f32) {
        if let Some(param) = LightningParam::from_property_type(property_type) {
            apply_lightning_param_value(self, param, value);
        }
    }
}

pub fn lightning_time_advance(ctx: &mut FrameContext) {
    advance_effect_time::<LightningEffect>(ctx);
}
