use cgmath::{Deg, Matrix3, Quaternion, Vector3};

use crate::asset::AssetStorage;
use crate::ecs::component::SceneCamera;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{ActiveSceneCamera, Camera, CameraPose};
use crate::ecs::systems::{
    compute_camera_backward, compute_camera_position, compute_camera_right, compute_camera_up,
};
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::scene::{spawn_scene_owner, SceneOwner};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum SceneCameraEvent {
    AddAtEditorCamera,
    ToggleCameraView,
    LookThrough(Entity),
}

impl UiCommand for SceneCameraEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            SceneCameraEvent::AddAtEditorCamera => add_scene_camera_at_editor_camera(world),
            SceneCameraEvent::ToggleCameraView => toggle_camera_view(world),
            SceneCameraEvent::LookThrough(entity) => look_through(world, entity),
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

fn toggle_camera_view(world: &mut World) {
    let view_enabled = world.resource::<ActiveSceneCamera>().view_enabled;
    if view_enabled {
        exit_camera_view(world);
    } else {
        enter_camera_view(world);
    }
}

fn enter_camera_view(world: &mut World) {
    let Some(entity) = world.resource::<ActiveSceneCamera>().entity else {
        return;
    };
    let Some(scene_camera) = world.get_component::<SceneCamera>(entity).cloned() else {
        return;
    };

    let mut editor_camera = world.resource_mut::<Camera>();
    let mut active = world.resource_mut::<ActiveSceneCamera>();
    active.saved_editor_pose = Some(editor_camera.pose());
    active.saved_fov_y = Some(editor_camera.fov_y.0);
    active.view_enabled = true;

    let distance = editor_camera.distance;
    editor_camera.set_pose(editor_pose_from_scene_camera(&scene_camera, distance));
    editor_camera.fov_y = Deg(scene_camera.fov_y_degrees);
}

fn exit_camera_view(world: &mut World) {
    let mut editor_camera = world.resource_mut::<Camera>();
    let mut active = world.resource_mut::<ActiveSceneCamera>();
    if let Some(pose) = active.saved_editor_pose.take() {
        editor_camera.set_pose(pose);
    }
    if let Some(fov_y) = active.saved_fov_y.take() {
        editor_camera.fov_y = Deg(fov_y);
    }
    active.view_enabled = false;
}

fn look_through(world: &mut World, entity: Entity) {
    if world.get_component::<SceneCamera>(entity).is_none() {
        return;
    }

    let view_enabled = world.resource::<ActiveSceneCamera>().view_enabled;
    if view_enabled {
        exit_camera_view(world);
    }
    world.resource_mut::<ActiveSceneCamera>().entity = Some(entity);
    enter_camera_view(world);
}

fn editor_pose_from_scene_camera(scene_camera: &SceneCamera, distance: f32) -> CameraPose {
    let [s, x, y, z] = scene_camera.rotation;
    let backward = Matrix3::from(Quaternion::new(s, x, y, z)).z;

    CameraPose {
        pivot: Vector3::from(scene_camera.position) - backward * distance,
        yaw: backward.x.atan2(backward.z),
        pitch: backward.y.clamp(-1.0, 1.0).asin(),
        distance,
    }
}

pub fn sync_scene_camera_to_editor_camera(world: &mut World) {
    let Some(entity) = camera_view_entity(&world.resource::<ActiveSceneCamera>()) else {
        return;
    };
    let editor_placement = scene_camera_from_editor_camera(&world.resource::<Camera>());

    if let Some(scene_camera) = world.get_component_mut::<SceneCamera>(entity) {
        scene_camera.position = editor_placement.position;
        scene_camera.rotation = editor_placement.rotation;
    }
    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
        (transform.translation, transform.rotation) = editor_placement.placement();
    }
}

fn camera_view_entity(active: &ActiveSceneCamera) -> Option<Entity> {
    active.entity.filter(|_| active.view_enabled)
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
    use cgmath::InnerSpace;

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

    fn toggle_camera_view(world: &mut World) {
        Box::new(SceneCameraEvent::ToggleCameraView).apply(
            world,
            &mut AssetStorage::new(),
            &GraphicsResources::default(),
        );
    }

    #[test]
    fn round_trip_editor_to_scene_camera_and_back_matches_yaw_pitch_position() {
        let mut camera = Camera::default();
        camera.pivot = Vector3::new(1.0, 2.0, -3.0);
        camera.yaw = 0.7;
        camera.pitch = -0.4;
        let original_pose = camera.pose();

        let scene_camera = scene_camera_from_editor_camera(&camera);
        let round_trip = editor_pose_from_scene_camera(&scene_camera, original_pose.distance);

        assert!((round_trip.yaw - original_pose.yaw).abs() < 1e-5);
        assert!((round_trip.pitch - original_pose.pitch).abs() < 1e-5);
        assert!((round_trip.pivot - original_pose.pivot).magnitude() < 1e-4);
        assert!(
            (Vector3::from(scene_camera.position) - compute_camera_position(&camera)).magnitude()
                < 1e-5
        );
    }

    #[test]
    fn toggle_on_off_restores_original_pose_and_fov() {
        let mut world = world_with_cameras();
        add_at_editor_camera(&mut world);

        world.resource_mut::<Camera>().yaw += 0.5;
        world.resource_mut::<Camera>().pitch += 0.3;
        world.resource_mut::<Camera>().fov_y = Deg(70.0);
        let (original_pose, original_fov_y) = {
            let camera = world.resource::<Camera>();
            (camera.pose(), camera.fov_y.0)
        };

        toggle_camera_view(&mut world);
        assert!(world.resource::<ActiveSceneCamera>().view_enabled);
        assert_ne!(world.resource::<Camera>().pose(), original_pose);
        assert_eq!(
            world.resource::<Camera>().fov_y.0,
            SceneCamera::default().fov_y_degrees
        );

        toggle_camera_view(&mut world);
        let restored = world.resource::<Camera>();

        assert_eq!(restored.pose(), original_pose, "pose not restored");
        assert!(
            (restored.fov_y.0 - original_fov_y).abs() < 1e-6,
            "fov_y not restored: {} vs {}",
            restored.fov_y.0,
            original_fov_y
        );
    }

    #[test]
    fn toggle_no_op_without_active_entity() {
        let mut world = world_with_cameras();

        let (original_pose, original_fov_y) = {
            let camera = world.resource::<Camera>();
            (camera.pose(), camera.fov_y.0)
        };

        toggle_camera_view(&mut world);

        let restored = world.resource::<Camera>();
        assert_eq!(
            restored.pose(),
            original_pose,
            "pose changed without active entity"
        );
        assert!(
            (restored.fov_y.0 - original_fov_y).abs() < 1e-6,
            "fov_y changed without active entity"
        );
        assert!(!world.resource::<ActiveSceneCamera>().view_enabled);
    }

    fn look_through(world: &mut World, entity: Entity) {
        Box::new(SceneCameraEvent::LookThrough(entity)).apply(
            world,
            &mut AssetStorage::new(),
            &GraphicsResources::default(),
        );
    }

    #[test]
    fn look_through_switches_active_and_enters_view() {
        let mut world = world_with_cameras();
        add_at_editor_camera(&mut world);
        let first = world.resource::<ActiveSceneCamera>().entity;

        add_at_editor_camera(&mut world);
        let second = world
            .entities_with::<SceneCamera>()
            .into_iter()
            .find(|&entity| Some(entity) != first)
            .expect("second scene camera");

        look_through(&mut world, second);
        let active = world.resource::<ActiveSceneCamera>();
        assert_eq!(active.entity, Some(second), "active not switched");
        assert!(active.view_enabled, "view not enabled");
    }

    #[test]
    fn look_through_during_camera_view_keeps_original_editor_pose() {
        let mut world = world_with_cameras();
        add_at_editor_camera(&mut world);
        let first = world.resource::<ActiveSceneCamera>().entity;
        world.resource_mut::<Camera>().yaw += 0.5;
        add_at_editor_camera(&mut world);
        let second = world
            .entities_with::<SceneCamera>()
            .into_iter()
            .find(|&entity| Some(entity) != first)
            .expect("second scene camera");
        world.resource_mut::<Camera>().yaw += 0.5;
        let original_pose = world.resource::<Camera>().pose();

        toggle_camera_view(&mut world);
        look_through(&mut world, second);
        assert_eq!(world.resource::<ActiveSceneCamera>().entity, Some(second));
        assert!(world.resource::<ActiveSceneCamera>().view_enabled);

        toggle_camera_view(&mut world);
        assert_eq!(world.resource::<Camera>().pose(), original_pose);
    }
}
