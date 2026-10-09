use crate::animation::decompose_transform;
use crate::asset::AssetStorage;
use crate::ecs::component::{CameraComponent, EntityIcon};
use crate::ecs::world::{Transform, World};
use crate::hooks::model_load::LoadedModel;
use cgmath::Deg;
use thyllore_importer_core::gltf::CameraProjection;

pub fn camera_import_spawn_loaded(world: &mut World, _: &AssetStorage, loaded: &LoadedModel) {
    let parent_entity = loaded.entity;
    let load_result = loaded.load_result;

    for (index, camera) in load_result.cameras.iter().enumerate() {
        let (translation, rotation, scale) = decompose_transform(&camera.world_transform);
        let transform = Transform {
            translation,
            rotation,
            scale,
        };

        let name = if camera.name.is_empty() {
            format!("Camera_{:02}", index + 1)
        } else {
            camera.name.clone()
        };

        let camera_component = match &camera.projection {
            CameraProjection::Perspective {
                yfov, znear, zfar, ..
            } => CameraComponent {
                fov_y: Deg(yfov.to_degrees()),
                near_plane: *znear,
                far_plane: *zfar,
                physical: Default::default(),
            },
            CameraProjection::Orthographic { znear, zfar, .. } => CameraComponent {
                near_plane: *znear,
                far_plane: Some(*zfar),
                ..CameraComponent::default()
            },
        };

        let entity = world
            .entity()
            .with_name(&name)
            .with_transform(transform)
            .with_visible(true)
            .with_parent(parent_entity)
            .with_editor_display(EntityIcon::Camera, false)
            .build();
        world.insert_component(entity, camera_component);

        log!(
            "Spawned camera entity '{}' from glTF node {}: entity_id={}",
            name,
            camera.node_index,
            entity
        );
    }
}

crate::model_load_hook!("camera_import", Rig, camera_import_spawn_loaded);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::world::{Entity, Parent};
    fn make_perspective_camera(
        world_transform: cgmath::Matrix4<f32>,
    ) -> thyllore_importer_core::gltf::LoadedCamera {
        thyllore_importer_core::gltf::LoadedCamera {
            node_index: 0,
            name: "Cam".into(),
            world_transform,
            projection: CameraProjection::Perspective {
                yfov: 0.6911,
                aspect_ratio: None,
                znear: 0.1,
                zfar: Some(100.0),
            },
        }
    }

    fn build_result(
        cameras: Vec<thyllore_importer_core::gltf::LoadedCamera>,
    ) -> thyllore_importer_core::ModelLoadResult {
        thyllore_importer_core::ModelLoadResult {
            cameras,
            ..Default::default()
        }
    }

    fn build_loaded<'a>(
        world: &mut World,
        result: &'a thyllore_importer_core::ModelLoadResult,
    ) -> LoadedModel<'a> {
        let entity = world.entity().build();
        LoadedModel {
            entity,
            load_result: result,
        }
    }

    #[test]
    fn test_spawn_loaded_cameras() {
        let mut world = World::new();

        let result = build_result(vec![make_perspective_camera(
            cgmath::Matrix4::from_translation(cgmath::Vector3::new(1.0, 2.0, 3.0)),
        )]);

        let loaded = build_loaded(&mut world, &result);
        camera_import_spawn_loaded(&mut world, &AssetStorage::default(), &loaded);

        let cameras: Vec<_> = world.iter_components::<CameraComponent>().collect();
        assert_eq!(cameras.len(), 1, "Expected exactly 1 camera entity");

        let (entity, camera) = &cameras[0];

        let transform = world
            .get_component::<Transform>(*entity)
            .expect("Camera entity should have a Transform");
        assert!((transform.translation.x - 1.0).abs() < 1e-6);
        assert!((transform.translation.y - 2.0).abs() < 1e-6);
        assert!((transform.translation.z - 3.0).abs() < 1e-6);

        let fov_deg = camera.fov_y.0;
        assert!(
            (fov_deg - 39.6).abs() < 0.1,
            "Expected fov_y ~39.6°, got {}",
            fov_deg
        );

        let parent = world
            .get_component::<Parent>(*entity)
            .expect("Camera entity should have a Parent");
        assert_eq!(parent.0, loaded.entity);
    }
}
