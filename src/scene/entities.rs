use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::motion_path_format::MOTION_PATH_SCENE_COMPONENT;
use super::scheduled_clip::SCHEDULED_CLIP_SCENE_COMPONENT;
use crate::animation::editable::EditableAnimationClip;
use crate::asset::AssetStorage;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::systems::scalar_clip_systems::{
    despawn_entity_with_clip, ensure_scalar_entity_clip, find_entity_clip_id,
};
use crate::ecs::world::{Entity, Name, World};
use crate::hooks::scene::{
    SceneComponentHook, SceneComponentHooks, SceneComponentRole, SceneValue,
};

/// A named entity stored as the persisted form of its components, keyed by type key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub name: String,
    #[serde(default)]
    pub components: BTreeMap<String, SceneValue>,
}

/// Scene components that belong to no effect: the clip an entity schedules and its motion path.
pub fn subscribe_scene_components(hooks: &mut SceneComponentHooks) {
    hooks.register(SCHEDULED_CLIP_SCENE_COMPONENT);
    hooks.register(MOTION_PATH_SCENE_COMPONENT);
}

/// Every entity an owner hook reports, with what each registered hook captures from it.
pub fn capture_scene_entities(world: &World) -> Vec<SceneEntity> {
    let Some(hooks) = world.get_resource::<SceneComponentHooks>() else {
        log_warn!("SceneComponentHooks resource missing: no scene entities captured");
        return Vec::new();
    };

    owned_entities(world, hooks.owners())
        .into_iter()
        .map(|entity| SceneEntity {
            name: entity_name(world, entity),
            components: hooks
                .ordered()
                .filter_map(|hook| {
                    (hook.capture)(world, entity).map(|value| (hook.type_key.to_string(), value))
                })
                .collect(),
        })
        .collect()
}

/// The clips scheduled by owned entities, so they can be re-registered after a library reset.
pub fn capture_scheduled_clips(world: &World) -> Vec<EditableAnimationClip> {
    let Some(hooks) = world.get_resource::<SceneComponentHooks>() else {
        return Vec::new();
    };
    let Some(library) = world.get_resource::<ClipLibrary>() else {
        return Vec::new();
    };

    let clip_ids: BTreeSet<_> = owned_entities(world, hooks.owners())
        .into_iter()
        .filter_map(|entity| find_entity_clip_id(world, entity))
        .collect();
    clip_ids
        .into_iter()
        .filter_map(|clip_id| library.get(clip_id).cloned())
        .collect()
}

/// Replaces every owned entity by the scene's entities; loading twice yields the same world.
pub fn apply_scene_entities(
    world: &mut World,
    assets: &mut AssetStorage,
    entities: &[SceneEntity],
) {
    let hooks: Vec<SceneComponentHook> = match world.get_resource::<SceneComponentHooks>() {
        Some(hooks) => hooks.ordered().copied().collect(),
        None => {
            log_warn!("SceneComponentHooks resource missing: scene entities not applied");
            return;
        }
    };

    let owners = hooks
        .iter()
        .filter(|hook| hook.role == SceneComponentRole::Owner);
    for entity in owned_entities(world, owners) {
        despawn_entity_with_clip(world, entity);
    }

    for scene_entity in entities {
        apply_scene_entity(world, assets, &hooks, scene_entity);
    }
}

fn apply_scene_entity(
    world: &mut World,
    assets: &mut AssetStorage,
    hooks: &[SceneComponentHook],
    scene_entity: &SceneEntity,
) {
    for type_key in scene_entity.components.keys() {
        if hooks.iter().all(|hook| hook.type_key != type_key) {
            log_warn!(
                "Unknown scene component {} on entity {}",
                type_key,
                scene_entity.name
            );
        }
    }

    let entity = world.entity().with_name(&scene_entity.name).build();
    let mut has_owner = false;
    for hook in hooks {
        let Some(value) = scene_entity.components.get(hook.type_key) else {
            continue;
        };
        match (hook.apply)(world, assets, entity, value) {
            Ok(()) => has_owner |= hook.role == SceneComponentRole::Owner,
            Err(error) => log_warn!(
                "Failed to apply scene component {} on entity {}: {:#}",
                hook.type_key,
                scene_entity.name,
                error
            ),
        }
    }

    if !has_owner {
        log_warn!(
            "Scene entity {} has no owner component and was skipped",
            scene_entity.name
        );
        world.despawn(entity);
        return;
    }
    ensure_scalar_entity_clip(world, assets, entity);
}

fn owned_entities<'a>(
    world: &World,
    owners: impl Iterator<Item = &'a SceneComponentHook>,
) -> BTreeSet<Entity> {
    owners.flat_map(|hook| (hook.entities)(world)).collect()
}

fn entity_name(world: &World, entity: Entity) -> String {
    world
        .get_component::<Name>(entity)
        .map(|name| name.0.clone())
        .unwrap_or_default()
}

#[cfg(test)]
pub(super) fn world_with_scene_hooks() -> World {
    let mut world = World::new();
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(all_scene_component_hooks());
    world
}

#[cfg(test)]
pub(super) fn all_scene_component_hooks() -> SceneComponentHooks {
    let mut hooks = SceneComponentHooks::default();
    subscribe_scene_components(&mut hooks);
    crate::effect::subscription::subscribe_scene_components(&mut hooks);
    hooks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::{
        AppliedFlameStyle, AppliedWaterPreset, FlameEffect, FlameParam, MotionPath,
        WaterTorusEffect, FLAME_DOMAIN,
    };
    use crate::ecs::systems::{spawn_flame_with_clip, spawn_water_with_clip, DEFAULT_FLAME_NAME};
    use crate::hooks::scene::{decode_scene_value, encode_scene_value};
    use crate::scene::scheduled_clip::ScheduledClip;
    use thyllore_anim_core::editable::{curve_add_keyframe, InterpolationType};

    fn flame_with_keyed_clip(world: &mut World, assets: &mut AssetStorage) -> Entity {
        let entity =
            spawn_flame_with_clip(world, assets, DEFAULT_FLAME_NAME, FlameEffect::default());
        let clip_id = find_entity_clip_id(world, entity).expect("flame has a clip");
        let mut library = world.resource_mut::<ClipLibrary>();
        let clip = library.get_mut(clip_id).expect("clip registered");
        let curve = clip.get_or_add_scalar_curve(FlameParam::Height.property_type());
        let key = curve_add_keyframe(curve, 1.0, 2.0);
        curve
            .get_keyframe_mut(key)
            .expect("key inserted")
            .interpolation = InterpolationType::Bezier;
        entity
    }

    #[test]
    fn hooks_are_registered_owners_first_and_include_every_core_component() {
        let hooks = all_scene_component_hooks();
        let keys = hooks.type_keys();

        let owner_count = hooks.owners().count();
        assert!(owner_count >= 3, "{keys:?}");
        assert!(keys[owner_count..].contains(&"clip"), "{keys:?}");
        assert!(keys[owner_count..].contains(&"motion_path"), "{keys:?}");
    }

    #[test]
    fn capture_writes_one_entity_per_owner_with_its_attachments() {
        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        let flame = flame_with_keyed_clip(&mut world, &mut assets);
        world.insert_component(
            flame,
            AppliedFlameStyle {
                name: "pillar".to_string(),
                version: 3,
            },
        );
        world.insert_component(
            flame,
            MotionPath {
                radius: 2.0,
                enabled: true,
                ..MotionPath::default()
            },
        );
        spawn_water_with_clip(
            &mut world,
            &mut assets,
            "Water",
            WaterTorusEffect::default(),
        );

        let entities = capture_scene_entities(&world);

        assert_eq!(entities.len(), 2);
        let flame_entity = &entities[0];
        assert_eq!(flame_entity.name, DEFAULT_FLAME_NAME);
        let keys: Vec<&String> = flame_entity.components.keys().collect();
        assert_eq!(keys, ["clip", "flame", "flame_style", "motion_path"]);
        let clip: ScheduledClip =
            decode_scene_value(&flame_entity.components["clip"]).expect("clip decodes");
        assert_eq!(clip.clip, FLAME_DOMAIN.name);
        let style: AppliedFlameStyle =
            decode_scene_value(&flame_entity.components["flame_style"]).expect("style decodes");
        assert_eq!(style.version, 3);
        assert_eq!(entities[1].name, "Water");
        assert!(entities[1].components.contains_key("water_torus"));
        assert!(!entities[1].components.contains_key("flame"));
    }

    #[test]
    fn apply_restores_entities_attachments_and_scheduled_clips() {
        let mut source = world_with_scene_hooks();
        let mut source_assets = AssetStorage::new();
        let flame = flame_with_keyed_clip(&mut source, &mut source_assets);
        source
            .get_component_mut::<FlameEffect>(flame)
            .expect("flame effect")
            .height = 7.5;
        source.insert_component(
            flame,
            AppliedFlameStyle {
                name: "pillar".to_string(),
                version: 3,
            },
        );
        let entities = capture_scene_entities(&source);
        let clips = capture_scheduled_clips(&source);
        assert_eq!(clips.len(), 1);

        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        crate::ecs::systems::clip_library_systems::clip_library_register_loaded(
            &mut world,
            &mut assets,
            clips,
        );
        apply_scene_entities(&mut world, &mut assets, &entities);

        let flames = world.query_flames();
        assert_eq!(flames.len(), 1);
        let effect = world
            .get_component::<FlameEffect>(flames[0])
            .expect("flame");
        assert_eq!(effect.height, 7.5);
        assert_eq!(
            world.get_component::<Name>(flames[0]).map(|n| n.0.as_str()),
            Some(DEFAULT_FLAME_NAME)
        );
        let style = world
            .get_component::<AppliedFlameStyle>(flames[0])
            .expect("style restored");
        assert_eq!((style.name.as_str(), style.version), ("pillar", 3));

        let clip_id = find_entity_clip_id(&world, flames[0]).expect("clip scheduled");
        let library = world.resource::<ClipLibrary>();
        let curve = library
            .get(clip_id)
            .expect("scheduled clip is in the library")
            .get_scalar_curve(FlameParam::Height.property_type())
            .expect("keyed curve restored");
        assert_eq!(curve.keyframes.len(), 1);
        assert_eq!(curve.keyframes[0].interpolation, InterpolationType::Bezier);
        assert_eq!(library.source_clips.len(), 1, "no orphan clip");
    }

    #[test]
    fn apply_twice_replaces_previous_entities_and_their_clips() {
        let mut source = world_with_scene_hooks();
        let mut source_assets = AssetStorage::new();
        spawn_flame_with_clip(
            &mut source,
            &mut source_assets,
            DEFAULT_FLAME_NAME,
            FlameEffect::default(),
        );
        let entities = capture_scene_entities(&source);

        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_scene_entities(&mut world, &mut assets, &entities);
        let first = world.query_flames()[0];
        apply_scene_entities(&mut world, &mut assets, &entities);

        let flames = world.query_flames();
        assert_eq!(flames.len(), 1);
        assert_ne!(flames[0], first);
        assert_eq!(world.resource::<ClipLibrary>().source_clips.len(), 1);
    }

    #[test]
    fn owner_without_scheduled_clip_still_gets_an_empty_clip_lane() {
        let entities = vec![SceneEntity {
            name: "Water".to_string(),
            components: BTreeMap::from([(
                "water_torus".to_string(),
                encode_scene_value(&WaterTorusEffect::default()).expect("encode"),
            )]),
        }];

        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_scene_entities(&mut world, &mut assets, &entities);

        let water = world.query_waters()[0];
        assert!(find_entity_clip_id(&world, water).is_some());
        assert!(world.get_component::<AppliedWaterPreset>(water).is_none());
    }

    #[test]
    fn entity_without_owner_component_is_skipped() {
        let entities = vec![SceneEntity {
            name: "orphan".to_string(),
            components: BTreeMap::from([
                (
                    "motion_path".to_string(),
                    encode_scene_value(&MotionPath::default()).expect("encode"),
                ),
                ("unknown_key".to_string(), SceneValue::Unit),
            ]),
        }];

        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_scene_entities(&mut world, &mut assets, &entities);

        assert!(world.iter_components::<Name>().next().is_none());
    }

    #[test]
    fn scene_entities_survive_a_ron_round_trip() {
        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        spawn_flame_with_clip(&mut world, &mut assets, "Flame", FlameEffect::default());
        let entities = capture_scene_entities(&world);

        let serialized = ron::ser::to_string_pretty(&entities, ron::ser::PrettyConfig::new())
            .expect("entities serialize to ron");
        let restored: Vec<SceneEntity> = ron::from_str(&serialized).expect("entities parse back");

        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].components, entities[0].components);
    }
}
