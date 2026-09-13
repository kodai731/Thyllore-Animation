use crate::app::FrameContext;
use crate::ecs::component::{apply_water_param_value, WaterParam, WaterTorusEffect};
use crate::ecs::resource::WaterRenderSettings;
use crate::ecs::systems::effect_time::{advance_effect_time, EffectTimeSources, TimedEffect};
use crate::ecs::world::{Entity, Transform, World};
use thyllore_anim_core::editable::PropertyType;

impl TimedEffect for WaterTorusEffect {
    type WorldInputs = ();

    fn entities(world: &World) -> Vec<Entity> {
        world.query_waters()
    }

    fn time_sources(world: &World, delta_time: f32) -> EffectTimeSources {
        let (batch_fixed_time, free_run_when_paused) = world
            .get_resource::<WaterRenderSettings>()
            .map(|settings| (settings.batch_fixed_time, settings.free_run_when_paused))
            .unwrap_or((None, WaterRenderSettings::default().free_run_when_paused));
        EffectTimeSources::collect(world, delta_time, batch_fixed_time, free_run_when_paused)
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
        if let Some(param) = WaterParam::from_property_type(property_type) {
            apply_water_param_value(self, param, value);
        }
    }
}

pub fn water_time_advance(ctx: &mut FrameContext) {
    advance_effect_time::<WaterTorusEffect>(ctx);
}
