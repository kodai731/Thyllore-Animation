use crate::asset::AssetStorage;
use crate::ecs::component::{LightningEffect, LIGHTNING_DOMAIN};
use crate::ecs::resource::HierarchyState;
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::scene::spawn_scene_owner;

pub const DEFAULT_LIGHTNING_NAME: &str = "Lightning";

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

    world.query_lightnings().first().copied()
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
