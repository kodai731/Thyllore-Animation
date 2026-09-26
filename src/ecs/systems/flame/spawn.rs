use crate::asset::AssetStorage;
use crate::ecs::component::{FlameEffect, FLAME_DOMAIN};
use crate::ecs::resource::{
    FlameHistorySnapshotState, FlameRenderSettings, FlameUIState, HierarchyState,
};
use crate::ecs::world::{Entity, World};
use crate::hooks::effect_spawn::EffectSpawnHook;
use crate::hooks::effect_ui_event::EffectUiQueue;
use crate::hooks::scene::spawn_scene_owner;

pub const DEFAULT_FLAME_NAME: &str = "Flame";

pub const FLAME_SPAWN_HOOK: EffectSpawnHook = EffectSpawnHook {
    key: "flame",
    max_instances: thyllore_effect_core::FLAME_MAX_INSTANCES,
    spawn: spawn_flame_instance,
    entities: flame_entities,
    default_in_empty_scene: true,
};

crate::effect_spawn_hook!(FLAME_SPAWN_HOOK);

fn flame_entities(world: &World) -> Vec<Entity> {
    world.entities_with::<FlameEffect>()
}

fn spawn_flame_instance(world: &mut World, assets: &mut AssetStorage, ordinal: usize) -> Entity {
    if ordinal == 0 {
        return spawn_flame_with_clip(world, assets, DEFAULT_FLAME_NAME, FlameEffect::default());
    }
    let effect = FlameEffect {
        position: cgmath::Vector3::new(1.5 * ordinal as f32, 0.0, 0.0),
        ..FlameEffect::default()
    };
    spawn_flame_with_clip(
        world,
        assets,
        &format!("{DEFAULT_FLAME_NAME} {}", ordinal + 1),
        effect,
    )
}

/// Spawns a flame as a regular scene entity so the hierarchy, inspector and transform gizmo
/// can all reach it through the same components they use for every other object.
pub fn spawn_flame(world: &mut World, name: &str, effect: FlameEffect) -> Entity {
    spawn_scene_owner(world, name, effect)
}

/// Spawn a flame entity together with its (empty) animation clip and schedule
/// instance, so every created flame is animatable and shows a Timeline clip
/// lane immediately instead of waiting for the first inserted key.
pub fn spawn_flame_with_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    name: &str,
    effect: FlameEffect,
) -> Entity {
    let entity = spawn_flame(world, name, effect);
    crate::ecs::systems::scalar_clip_systems::ensure_entity_clip(
        world,
        assets,
        entity,
        &FLAME_DOMAIN,
    );
    entity
}

/// The flame the UI and the flame events act on: the selected entity when it is a flame,
/// otherwise the first one. Keeping this in one place is what lets the hierarchy selection
/// stay the single source of truth.
pub fn resolve_selected_flame(world: &World) -> Option<Entity> {
    let selected = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity);

    if let Some(entity) = selected {
        if world.get_component::<FlameEffect>(entity).is_some() {
            return Some(entity);
        }
    }

    world.entities_with::<FlameEffect>().first().copied()
}

fn insert_flame_default_resources(world: &mut World) {
    if !world.contains_resource::<FlameRenderSettings>() {
        world.insert_resource(FlameRenderSettings::default());
    }
    if !world.contains_resource::<FlameHistorySnapshotState>() {
        world.insert_resource(FlameHistorySnapshotState::default());
    }
    if !world.contains_resource::<FlameUIState>() {
        world.insert_resource(FlameUIState::default());
    }
    if !world.contains_resource::<EffectUiQueue<super::ui_command::FlameUiCommand>>() {
        world.insert_resource(EffectUiQueue::<super::ui_command::FlameUiCommand>::default());
    }
}

crate::effect_default_resource!("flame", insert_flame_default_resources);

crate::ui_event_hook!(
    FLAME_SPAWN_HOOK.key,
    super::ui_apply::dispatch_flame_ui_events
);
