use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

/// The model file an entity is loaded from; its mesh entities are the entity's children.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSource {
    pub path: String,
}

impl ModelSource {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

impl SceneComponent for ModelSource {
    const TYPE_KEY: &'static str = "model_source";
    const PERSISTED_FIELDS: &'static [&'static str] = &["path"];
}

crate::scene_owner!(ModelSource {
    icon: crate::ecs::component::EntityIcon::Model,
    placement: |_| (Vec3::ZERO, Quat::IDENTITY),
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetStorage;
    use crate::ecs::world::Name;
    use crate::hooks::scene::spawn_scene_owner;
    use crate::scene::{apply_scene_entities, capture_scene_entities, world_with_scene_hooks};

    #[test]
    fn model_source_entity_roundtrips_through_scene_entities() {
        let mut world = world_with_scene_hooks();
        spawn_scene_owner(
            &mut world,
            "avatar",
            ModelSource::new("models/avatar/avatar.fbx"),
        );

        let captured = capture_scene_entities(&world);
        assert_eq!(captured.len(), 1);
        assert!(captured[0].components.contains_key(ModelSource::TYPE_KEY));

        let mut restored = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_scene_entities(&mut restored, &mut assets, &captured);

        let sources: Vec<_> = restored.iter_components::<ModelSource>().collect();
        assert_eq!(sources.len(), 1);
        let (entity, source) = sources[0];
        assert_eq!(source.path, "models/avatar/avatar.fbx");
        assert_eq!(
            restored.get_component::<Name>(entity).map(|n| n.0.as_str()),
            Some("avatar")
        );
    }
}
