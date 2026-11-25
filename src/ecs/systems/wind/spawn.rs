use crate::asset::AssetStorage;
use crate::ecs::component::{WindTornadoEffect, WIND_DOMAIN};
use crate::ecs::world::{Entity, World};
use crate::hooks::effect_spawn::EffectSpawnHook;
use crate::hooks::scene::spawn_scene_owner;
use glam::Vec3;

pub const DEFAULT_WIND_NAME: &str = "Wind";

pub const WIND_SPAWN_HOOK: EffectSpawnHook = EffectSpawnHook {
    key: "wind",
    max_instances: thyllore_effect_core::wind::WIND_MAX_INSTANCES,
    spawn: spawn_wind_instance,
    entities: wind_entities,
    default_in_empty_scene: false,
};

crate::effect_spawn_hook!(WIND_SPAWN_HOOK);

fn wind_entities(world: &World) -> Vec<Entity> {
    world.entities_with::<WindTornadoEffect>()
}

fn spawn_wind_instance(world: &mut World, assets: &mut AssetStorage, ordinal: usize) -> Entity {
    let effect = WindTornadoEffect {
        position: Vec3::new(-2.5 * ordinal as f32, 0.0, 0.0),
        ..WindTornadoEffect::default()
    };
    spawn_wind_with_clip(
        world,
        assets,
        &format!("{DEFAULT_WIND_NAME} {}", ordinal + 1),
        effect,
    )
}

pub fn spawn_wind(world: &mut World, name: &str, effect: WindTornadoEffect) -> Entity {
    spawn_scene_owner(world, name, effect)
}

pub fn spawn_wind_with_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    name: &str,
    effect: WindTornadoEffect,
) -> Entity {
    let entity = spawn_wind(world, name, effect);
    crate::ecs::systems::ensure_entity_clip(world, assets, entity, &WIND_DOMAIN);
    entity
}
