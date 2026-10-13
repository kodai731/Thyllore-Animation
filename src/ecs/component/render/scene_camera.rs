use cgmath::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneCamera {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
    pub resolution: [u32; 2],
}

impl Default for SceneCamera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            fov_y_degrees: 45.0,
            near: 0.1,
            far: 1000.0,
            resolution: [1920, 1080],
        }
    }
}

impl SceneComponent for SceneCamera {
    const TYPE_KEY: &'static str = "scene_camera";
    const PERSISTED_FIELDS: &'static [&'static str] = &[
        "position",
        "rotation",
        "fov_y_degrees",
        "near",
        "far",
        "resolution",
    ];
}

crate::scene_owner!(SceneCamera {
    icon: crate::ecs::component::EntityIcon::Camera,
    placement: |camera| (
        Vector3::from(camera.position),
        Quaternion::new(
            camera.rotation[0],
            camera.rotation[1],
            camera.rotation[2],
            camera.rotation[3],
        ),
    ),
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::scene::spawn_scene_owner;

    #[test]
    fn scene_camera_scene_roundtrip_keeps_every_field() {
        let dir = std::env::temp_dir().join("thyllore_scene_camera_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let scenes_dir = dir.join("scenes");
        std::fs::create_dir_all(&scenes_dir).unwrap();
        let scene_path = scenes_dir.join("test.scene.ron");

        let camera = SceneCamera {
            position: [1.0, 2.0, 3.0],
            rotation: [0.5, 0.5, 0.5, 0.5],
            fov_y_degrees: 60.0,
            near: 0.5,
            far: 500.0,
            resolution: [3840, 2160],
        };
        let mut world = crate::scene::world_with_scene_hooks();
        spawn_scene_owner(&mut world, "Camera", camera.clone());

        crate::scene::save_scene(&scene_path, &world).unwrap();
        let loaded = crate::scene::load_scene(&scene_path).unwrap();
        let mut restored = crate::scene::world_with_scene_hooks();
        let mut restored_assets = crate::asset::AssetStorage::new();
        crate::scene::apply_loaded_scene_to_world(&loaded, &mut restored, &mut restored_assets);

        let cameras = restored.entities_with::<SceneCamera>();
        assert_eq!(cameras.len(), 1);
        assert_eq!(
            *restored.get_component::<SceneCamera>(cameras[0]).unwrap(),
            camera
        );
    }
}
