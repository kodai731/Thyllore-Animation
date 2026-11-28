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
    use crate::scene::{
        apply_loaded_scene_to_world, load_scene, save_scene, world_with_scene_hooks,
    };

    #[test]
    fn model_source_entity_roundtrips_through_the_scene_file() {
        let scenes_dir = std::env::temp_dir().join("thyllore_model_source_scene/scenes");
        std::fs::create_dir_all(&scenes_dir).unwrap();
        let scene_path = scenes_dir.join("test.scene.ron");
        let mut world = world_with_scene_hooks();
        spawn_scene_owner(
            &mut world,
            "avatar",
            ModelSource::new("models/avatar/avatar.fbx"),
        );

        save_scene(&scene_path, &world).unwrap();
        let loaded = load_scene(&scene_path).unwrap();
        let mut restored = world_with_scene_hooks();
        let mut assets = AssetStorage::new();
        apply_loaded_scene_to_world(&loaded, &mut restored, &mut assets);

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
