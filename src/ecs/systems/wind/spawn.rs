use crate::asset::AssetStorage;
use crate::ecs::component::{EditorDisplay, EntityIcon, WindTornadoEffect, WIND_DOMAIN};
use crate::ecs::resource::HierarchyState;
use crate::ecs::world::{Entity, GlobalTransform, Transform, World};

pub const DEFAULT_WIND_NAME: &str = "Wind";

pub fn spawn_wind(world: &mut World, name: &str, effect: WindTornadoEffect) -> Entity {
    let entity = world.entity().with_name(name).build();
    attach_wind(world, entity, effect);
    entity
}

/// Turns a named entity into a wind: transform, hierarchy icon and the effect component.
pub fn attach_wind(world: &mut World, entity: Entity, effect: WindTornadoEffect) {
    world.insert_component(
        entity,
        Transform {
            translation: effect.position,
            rotation: effect.rotation,
            ..Default::default()
        },
    );
    world.insert_component(entity, GlobalTransform::new());
    world.insert_component(entity, EditorDisplay::new(EntityIcon::Wind));

    world.insert_component(entity, effect);
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

    world.query_winds().first().copied()
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
