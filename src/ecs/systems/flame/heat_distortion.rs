use crate::ecs::component::{FlameEffect, HeatPlume};
use crate::ecs::resource::{HeatDistortion, HeatDistortionSource};
use crate::ecs::world::World;

/// The first flame carrying a `HeatPlume` drives the tonemap heat haze.
pub fn publish_heat_distortion(world: &mut World) {
    let active = world
        .entities_with::<FlameEffect>()
        .into_iter()
        .find_map(|entity| {
            let effect = world.get_component::<FlameEffect>(entity)?;
            let plume = world.get_component::<HeatPlume>(entity)?;
            Some(HeatDistortion {
                origin: [effect.position.x, effect.position.y, effect.position.z],
                temperature: plume.plume_temperature,
                width_base: plume.width_base,
                width_slope: plume.width_slope,
                distortion_gain: plume.distortion_gain,
                height: plume.plume_height,
                time: effect.time,
                turbulence_amp: plume.turbulence_amp,
                wind_direction: [effect.wind.direction.x, effect.wind.direction.y],
                rise_speed: effect.warp.rise_speed,
            })
        });
    world.insert_resource(HeatDistortionSource { active });
}
