use super::ui_command::WaterUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::{WaterTorusEffect, WATER_DOMAIN};
use crate::ecs::resource::{HierarchyState, WaterHistorySnapshotState, WaterRenderSettings};
use crate::ecs::world::{Entity, World};
use crate::hooks::effect_spawn::EffectSpawnHook;
use crate::hooks::effect_ui_event::EffectUiQueue;
use crate::hooks::scene::spawn_scene_owner;

pub const DEFAULT_WATER_NAME: &str = "Water";

pub const WATER_SPAWN_HOOK: EffectSpawnHook = EffectSpawnHook {
    key: "water",
    max_instances: thyllore_effect_core::WATER_MAX_INSTANCES,
    spawn: spawn_water_instance,
    entities: water_entities,
    default_in_empty_scene: false,
};

crate::effect_spawn_hook!(WATER_SPAWN_HOOK);

fn water_entities(world: &World) -> Vec<Entity> {
    world.entities_with::<WaterTorusEffect>()
}

fn spawn_water_instance(world: &mut World, assets: &mut AssetStorage, ordinal: usize) -> Entity {
    let effect = WaterTorusEffect {
        position: cgmath::Vector3::new(0.0, -0.5, 2.5 * ordinal as f32),
        ..WaterTorusEffect::default()
    };
    spawn_water_with_clip(
        world,
        assets,
        &format!("{DEFAULT_WATER_NAME} {}", ordinal + 1),
        effect,
    )
}

/// Spawns a water as a regular scene entity so the hierarchy, inspector and transform gizmo
/// can all reach it through the same components they use for every other object.
pub fn spawn_water(world: &mut World, name: &str, effect: WaterTorusEffect) -> Entity {
    spawn_scene_owner(world, name, effect)
}

/// Spawn a water entity together with its (empty) animation clip and schedule
/// instance, so every created water is animatable and shows a Timeline clip
/// lane immediately instead of waiting for the first inserted key.
pub fn spawn_water_with_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    name: &str,
    effect: WaterTorusEffect,
) -> Entity {
    let entity = spawn_water(world, name, effect);
    crate::ecs::systems::scalar_clip_systems::ensure_entity_clip(
        world,
        assets,
        entity,
        &WATER_DOMAIN,
    );
    entity
}

/// The water the UI and the water events act on: the selected entity when it is a water,
/// otherwise the first one. Keeping this in one place is what lets the hierarchy selection
/// stay the single source of truth.
pub fn resolve_selected_water(world: &World) -> Option<Entity> {
    let selected = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity);

    if let Some(entity) = selected {
        if world.get_component::<WaterTorusEffect>(entity).is_some() {
            return Some(entity);
        }
    }

    world.entities_with::<WaterTorusEffect>().first().copied()
}

fn insert_water_default_resources(world: &mut World) {
    if !world.contains_resource::<WaterRenderSettings>() {
        world.insert_resource(WaterRenderSettings::default());
    }
    if !world.contains_resource::<WaterHistorySnapshotState>() {
        world.insert_resource(WaterHistorySnapshotState::default());
    }
    if !world.contains_resource::<EffectUiQueue<WaterUiCommand>>() {
        world.insert_resource(EffectUiQueue::<WaterUiCommand>::default());
    }
}

crate::effect_default_resource!("water", insert_water_default_resources);

crate::ui_event_hook!(
    WATER_SPAWN_HOOK.key,
    super::ui_apply::dispatch_water_ui_events
);
