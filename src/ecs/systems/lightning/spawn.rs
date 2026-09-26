use crate::asset::AssetStorage;
use crate::ecs::component::{LightningEffect, LIGHTNING_DOMAIN};
use crate::ecs::resource::{HierarchyState, LightningRenderSettings};
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::effect_spawn::EffectSpawnHook;
use crate::hooks::scene::spawn_scene_owner;

pub const DEFAULT_LIGHTNING_NAME: &str = "Lightning";
pub const DEFAULT_LIGHTNING_PRESET: &str = "bolt";

pub const LIGHTNING_SPAWN_HOOK: EffectSpawnHook = EffectSpawnHook {
    key: "lightning",
    max_instances: thyllore_effect_core::LIGHTNING_MAX_INSTANCES,
    spawn: spawn_lightning_instance,
    entities: lightning_entities,
    default_in_empty_scene: false,
};

crate::effect_spawn_hook!(LIGHTNING_SPAWN_HOOK);

fn lightning_entities(world: &World) -> Vec<Entity> {
    world.entities_with::<LightningEffect>()
}

fn spawn_lightning_instance(
    world: &mut World,
    assets: &mut AssetStorage,
    ordinal: usize,
) -> Entity {
    let mut effect = LightningEffect {
        position: cgmath::Vector3::new(2.5 * ordinal as f32, 0.0, 0.0),
        ..LightningEffect::default()
    };
    thyllore_effect_core::apply_lightning_preset(&mut effect, DEFAULT_LIGHTNING_PRESET);
    let entity = spawn_lightning_with_clip(
        world,
        assets,
        &format!("{DEFAULT_LIGHTNING_NAME} {}", ordinal + 1),
        effect,
    );
    super::preset::record_lightning_preset(world, entity, DEFAULT_LIGHTNING_PRESET);
    entity
}

pub fn spawn_lightning(world: &mut World, name: &str, effect: LightningEffect) -> Entity {
    spawn_scene_owner(world, name, effect)
}

pub fn spawn_lightning_with_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    name: &str,
    effect: LightningEffect,
) -> Entity {
    let entity = spawn_lightning(world, name, effect);
    crate::ecs::systems::scalar_clip_systems::ensure_entity_clip(
        world,
        assets,
        entity,
        &LIGHTNING_DOMAIN,
    );
    entity
}

/// The lightning the UI and the lightning events act on: the selected entity when it is a
/// lightning, otherwise the first one.
pub fn resolve_selected_lightning(world: &World) -> Option<Entity> {
    let selected = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity);

    if let Some(entity) = selected {
        if world.get_component::<LightningEffect>(entity).is_some() {
            return Some(entity);
        }
    }

    world.entities_with::<LightningEffect>().first().copied()
}

pub fn write_lightning_transform(
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

fn insert_lightning_default_resources(world: &mut World) {
    if !world.contains_resource::<LightningRenderSettings>() {
        world.insert_resource(LightningRenderSettings::default());
    }
}

crate::effect_default_resource!("lightning", insert_lightning_default_resources);

crate::effect_ui_event_hook!(
    LIGHTNING_SPAWN_HOOK.key,
    super::ui_apply::apply_lightning_ui_command
);
