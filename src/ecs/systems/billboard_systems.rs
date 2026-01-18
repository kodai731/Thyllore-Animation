use cgmath::{InnerSpace, Vector3};

use crate::ecs::components::CameraState;
use crate::ecs::world::World;

pub fn billboard_system(world: &mut World, camera: &CameraState) {
    let billboards = world.query_billboards();

    for entity in billboards {
        let Some(behavior) = world.billboard_behaviors.get(&entity) else {
            continue;
        };

        if !behavior.always_face_camera {
            continue;
        }

        let Some(transform) = world.transforms.get_mut(&entity) else {
            continue;
        };

        let position = transform.translation;
        let to_camera = camera.position - position;

        if to_camera.magnitude() > 0.001 {
            let forward = to_camera.normalize();
            let up = Vector3::new(0.0, 1.0, 0.0);
            let right = up.cross(forward).normalize();
            let adjusted_up = forward.cross(right);

            transform.rotation =
                cgmath::Quaternion::from(cgmath::Matrix3::from_cols(right, adjusted_up, forward));
        }
    }
}
