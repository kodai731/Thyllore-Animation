use cgmath::{Matrix3, Quaternion};

use crate::asset::AssetStorage;
use crate::ecs::component::SceneCamera;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{ActiveSceneCamera, Camera};
use crate::ecs::systems::{
    compute_camera_backward, compute_camera_position, compute_camera_right, compute_camera_up,
};
use crate::ecs::world::World;
use crate::hooks::scene::spawn_scene_owner;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum SceneCameraEvent {
    AddAtEditorCamera,
}

impl UiCommand for SceneCameraEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            SceneCameraEvent::AddAtEditorCamera => add_scene_camera_at_editor_camera(world),
        }
    }
}

fn add_scene_camera_at_editor_camera(world: &mut World) {
    let scene_camera = scene_camera_from_editor_camera(&world.resource::<Camera>());
    let entity = spawn_scene_owner(world, "Camera", scene_camera);

    let mut active = world.resource_mut::<ActiveSceneCamera>();
    if active.entity.is_none() {
        active.entity = Some(entity);
    }
}

fn scene_camera_from_editor_camera(camera: &Camera) -> SceneCamera {
    let orientation = Matrix3::from_cols(
        compute_camera_right(camera),
        compute_camera_up(camera),
        compute_camera_backward(camera),
    );
    let rotation = Quaternion::from(orientation);

    SceneCamera {
        position: compute_camera_position(camera).into(),
        rotation: [rotation.s, rotation.v.x, rotation.v.y, rotation.v.z],
        ..SceneCamera::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_cameras() -> World {
        let mut world = crate::scene::world_with_scene_hooks();
        world.insert_resource(Camera::default());
        world.insert_resource(ActiveSceneCamera::default());
        world
    }

    fn add_at_editor_camera(world: &mut World) {
        Box::new(SceneCameraEvent::AddAtEditorCamera).apply(
            world,
            &mut AssetStorage::new(),
            &GraphicsResources::default(),
        );
    }

    #[test]
    fn add_at_editor_camera_activates_only_the_first_scene_camera() {
        let mut world = world_with_cameras();

        add_at_editor_camera(&mut world);
        let first = world
            .resource::<ActiveSceneCamera>()
            .entity
            .expect("first scene camera becomes active");

        add_at_editor_camera(&mut world);
        assert_eq!(world.resource::<ActiveSceneCamera>().entity, Some(first));
        assert_eq!(world.entities_with::<SceneCamera>().len(), 2);
    }
}
