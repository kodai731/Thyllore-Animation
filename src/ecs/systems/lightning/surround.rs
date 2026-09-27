use crate::ecs::component::LightningEffect;
use crate::ecs::resource::LightningSurroundClock;
use crate::ecs::resource::{
    DischargeEvent, DischargeEvents, FlashLightSource, FlashLightState, ImpactDecal,
};
use crate::ecs::world::World;
use crate::ecs::FrameContext;
use thyllore_effect_core::lightning::analytic::surround::{
    discharge_light, discharge_started_between, impact_glow,
};

pub fn publish_lightning_surround(world: &mut World) {
    if !world.contains_resource::<LightningSurroundClock>() {
        world.insert_resource(LightningSurroundClock::default());
    }

    let (Some(mut clock), Some(mut flash), Some(mut events)) = (
        world.get_resource_mut::<LightningSurroundClock>(),
        world.get_resource_mut::<FlashLightState>(),
        world.get_resource_mut::<DischargeEvents>(),
    ) else {
        return;
    };

    let mut brightest_light: Option<FlashLightSource> = None;
    let mut strongest_impact: Option<ImpactDecal> = None;

    for entity in world.entities_with::<LightningEffect>() {
        let Some(effect) = world.get_component::<LightningEffect>(entity) else {
            continue;
        };

        let current_time = effect.time;
        let previous_time = clock
            .previous_times
            .get(&entity)
            .copied()
            .unwrap_or(current_time);

        if let Some(event) = discharge_started_between(effect, previous_time, current_time) {
            events.push(DischargeEvent {
                entity,
                burst_index: event.burst_index,
                start_time: event.start_time,
                origin: event.origin,
                target: event.target,
                sound_speed: effect.surround.thunder_speed,
            });
        }

        if let Some(light) = discharge_light(effect, current_time) {
            if brightest_light.map_or(true, |brightest| light.intensity > brightest.intensity) {
                brightest_light = Some(FlashLightSource {
                    position: light.position,
                    intensity: light.intensity,
                    color: light.color,
                });
            }
        }

        if let Some(glow) = impact_glow(effect, current_time) {
            if strongest_impact.map_or(true, |strongest| glow.strength > strongest.strength) {
                strongest_impact = Some(ImpactDecal {
                    position: glow.position,
                    strength: glow.strength,
                    radius: glow.radius,
                });
            }
        }

        clock.previous_times.insert(entity, current_time);
    }

    flash.light = brightest_light;
    flash.impact = strongest_impact;
}

fn lightning_surround_advance(ctx: &mut FrameContext) {
    publish_lightning_surround(ctx.world);
}

crate::frame_prep_hook!("lightning_surround", Advance, lightning_surround_advance);
