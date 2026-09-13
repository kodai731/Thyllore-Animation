use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

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
pub(crate) fn world_with_scene_hooks() -> World {
    let mut world = World::new();
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(SceneComponentHooks::collect().expect("unique scene keys"));
    world.insert_resource(
        crate::hooks::scene_resource::SceneResourceHooks::collect().expect("unique resource keys"),
    );
    world
}

#[cfg(test)]
pub(crate) mod test_support {
    use cgmath::Quaternion;
    use serde::{Deserialize, Serialize};
    use thyllore_scene_core::SceneComponent;

    /// Test-only owner so scene tests never depend on a concrete effect.
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct ProbeOwner {
        pub position: [f32; 3],
        pub level: f32,
    }

    impl SceneComponent for ProbeOwner {
        const TYPE_KEY: &'static str = "test_probe_owner";
        const PERSISTED_FIELDS: &'static [&'static str] = &["position", "level"];
    }

    crate::scene_owner!(ProbeOwner {
        icon: Empty,
        placement: |p| (p.position.into(), Quaternion::new(1.0, 0.0, 0.0, 0.0)),
    });

    /// Test-only attachment restored by insertion.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct ProbeLabel {
        pub label: String,
    }

    impl SceneComponent for ProbeLabel {
        const TYPE_KEY: &'static str = "test_probe_label";
        const PERSISTED_FIELDS: &'static [&'static str] = &["label"];
    }

    crate::scene_attachment!(ProbeLabel);
}

#[cfg(test)]
mod tests {
    use super::test_support::{ProbeLabel, ProbeOwner};
    use super::*;
    use crate::ecs::world::Transform;
    use crate::hooks::scene::{
        decode_scene_value, encode_scene_value, entities_with, spawn_scene_owner,
    };
    use thyllore_scene_core::SceneComponent;

    fn spawn_probe(world: &mut World, name: &str, level: f32) -> Entity {
        spawn_scene_owner(
            world,
            name,
            ProbeOwner {
                position: [1.0, 2.0, 3.0],
                level,
            },
        )
    }

    #[test]
    fn hooks_are_registered_owners_first_and_include_every_core_component() {
        let hooks = SceneComponentHooks::collect().expect("unique scene keys");
        let keys = hooks.type_keys();

        let owner_count = hooks.owners().count();
        assert!(
            keys[..owner_count].contains(&ProbeOwner::TYPE_KEY),
            "{keys:?}"
        );
        assert!(keys[owner_count..].contains(&"clip"), "{keys:?}");
        assert!(keys[owner_count..].contains(&"motion_path"), "{keys:?}");
        assert!(
            keys[owner_count..].contains(&ProbeLabel::TYPE_KEY),
            "{keys:?}"
        );
    }

    #[test]
    fn capture_writes_one_entity_per_owner_with_its_attachments() {
        let mut world = world_with_scene_hooks();
        let first = spawn_probe(&mut world, "first", 0.25);
        world.insert_component(
            first,
            ProbeLabel {
                label: "tagged".to_string(),
            },
        );
        spawn_probe(&mut world, "second", 0.5);

        let entities = capture_scene_entities(&world);

        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].name, "first");
        let keys: Vec<&String> = entities[0].components.keys().collect();
        assert_eq!(keys, [ProbeLabel::TYPE_KEY, ProbeOwner::TYPE_KEY]);
        let label: ProbeLabel =
            decode_scene_value(&entities[0].components[ProbeLabel::TYPE_KEY]).expect("decodes");
        assert_eq!(label.label, "tagged");
        assert_eq!(entities[1].name, "second");
        assert!(!entities[1].components.contains_key(ProbeLabel::TYPE_KEY));
    }

    #[test]
    fn apply_restores_owner_placement_name_and_attachments() {
        let mut source = world_with_scene_hooks();
        let probe = spawn_probe(&mut source, "probe", 7.5);
        source.insert_component(
            probe,
            ProbeLabel {
                label: "tagged".to_string(),
            },
        );
        let entities = capture_scene_entities(&source);

        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_scene_entities(&mut world, &mut assets, &entities);

        let probes: Vec<_> = world.iter_components::<ProbeOwner>().collect();
        assert_eq!(probes.len(), 1);
        let (entity, owner) = probes[0];
        assert_eq!(owner.level, 7.5);
        assert_eq!(
            world.get_component::<Name>(entity).map(|n| n.0.as_str()),
            Some("probe")
        );
        assert_eq!(
            world
                .get_component::<Transform>(entity)
                .map(|t| t.translation),
            Some(cgmath::Vector3::new(1.0, 2.0, 3.0))
        );
        assert_eq!(
            world
                .get_component::<ProbeLabel>(entity)
                .map(|l| l.label.as_str()),
            Some("tagged")
        );
    }

    #[test]
    fn apply_twice_replaces_previous_owner_entities() {
        let mut source = world_with_scene_hooks();
        spawn_probe(&mut source, "probe", 1.0);
        let entities = capture_scene_entities(&source);

        let mut world = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_scene_entities(&mut world, &mut assets, &entities);
        let first = entities_with::<ProbeOwner>(&world)[0];
        apply_scene_entities(&mut world, &mut assets, &entities);

        let probes = entities_with::<ProbeOwner>(&world);
        assert_eq!(probes.len(), 1);
        assert_ne!(probes[0], first);
    }

    #[test]
    fn entity_without_owner_component_is_skipped() {
        let entities = vec![SceneEntity {
            name: "orphan".to_string(),
            components: BTreeMap::from([
                (
                    ProbeLabel::TYPE_KEY.to_string(),
                    encode_scene_value(&ProbeLabel {
                        label: "x".to_string(),
                    })
                    .expect("encode"),
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
        spawn_probe(&mut world, "probe", 0.125);
        let entities = capture_scene_entities(&world);

        let serialized = ron::ser::to_string_pretty(&entities, ron::ser::PrettyConfig::new())
            .expect("entities serialize to ron");
        let restored: Vec<SceneEntity> = ron::from_str(&serialized).expect("entities parse back");

        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].components, entities[0].components);
    }
}
