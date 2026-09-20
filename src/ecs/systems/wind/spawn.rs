use crate::asset::AssetStorage;
use crate::ecs::component::{WindTornadoEffect, WIND_DOMAIN};
use crate::ecs::resource::{HierarchyState, WindRenderSettings};
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::effect_spawn::EffectSpawnHook;
use crate::hooks::scene::spawn_scene_owner;

pub const DEFAULT_WIND_NAME: &str = "Wind";

pub const WIND_SPAWN_HOOK: EffectSpawnHook = EffectSpawnHook {
    key: "wind",
    max_instances: thyllore_effect_core::WIND_MAX_INSTANCES,
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
        position: cgmath::Vector3::new(-2.5 * ordinal as f32, 0.0, 0.0),
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
    crate::ecs::systems::scalar_clip_systems::ensure_entity_clip(
        world,
        assets,
        entity,
        &WIND_DOMAIN,
    );
    entity
}

/// The wind the UI and the wind events act on: the selected entity when it is a wind,
/// otherwise the first one.
pub fn resolve_selected_wind(world: &World) -> Option<Entity> {
    let selected = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity);

    if let Some(entity) = selected {
        if world.get_component::<WindTornadoEffect>(entity).is_some() {
            return Some(entity);
        }
    }

    world.entities_with::<WindTornadoEffect>().first().copied()
}

pub fn write_wind_transform(
    world: &mut World,
    entity: Entity,
    translation: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
) {
    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
        transform.translation = translation;
        transform.rotation = rotation;
    }
}

fn insert_wind_default_resources(world: &mut World) {
    if !world.contains_resource::<WindRenderSettings>() {
        world.insert_resource(WindRenderSettings::default());
    }
}

crate::effect_default_resource!("wind", insert_wind_default_resources);

crate::effect_ui_event_hook!(WIND_SPAWN_HOOK.key, super::ui_apply::apply_wind_ui_command);
