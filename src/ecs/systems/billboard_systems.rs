use anyhow::Result;
use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

use crate::ecs::component::{CameraState, RenderInfo};
use crate::ecs::resource::billboard::{
    BillboardData, BillboardMesh, BillboardRenderState, BillboardTransform, BillboardVertex,
};
use crate::ecs::world::{BillboardBehavior, Transform, World};
use crate::vulkanr::VulkanBackend;

pub fn create_billboard() -> BillboardData {
    let billboard_size = 0.5;
    let vertices = vec![
        BillboardVertex {
            pos: [-billboard_size, -billboard_size, 0.0],
            tex_coord: [0.0, 1.0],
        },
        BillboardVertex {
            pos: [billboard_size, -billboard_size, 0.0],
            tex_coord: [1.0, 1.0],
        },
        BillboardVertex {
            pos: [billboard_size, billboard_size, 0.0],
            tex_coord: [1.0, 0.0],
        },
        BillboardVertex {
            pos: [-billboard_size, billboard_size, 0.0],
            tex_coord: [0.0, 0.0],
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    BillboardData {
        mesh: BillboardMesh {
            vertices,
            indices,
            vertex_buffer_handle: Default::default(),
            index_buffer_handle: Default::default(),
        },
        transform: None,
        render_info: RenderInfo::default(),
        render_state: BillboardRenderState::default(),
    }
}

pub fn create_billboard_transform(position: Vec3) -> BillboardTransform {
    BillboardTransform {
        position,
        model_matrix: Mat4::from_translation(position),
    }
}

pub fn billboard_system(world: &mut World, camera: &CameraState) {
    let billboards = world.query_billboards();

    for entity in billboards {
        let Some(behavior) = world.get_component::<BillboardBehavior>(entity) else {
            continue;
        };

        if !behavior.always_face_camera {
            continue;
        }

        let Some(transform) = world.get_component_mut::<Transform>(entity) else {
            continue;
        };

        let position = transform.translation;
        let to_camera = camera.position - position;

        if to_camera.length() > 0.001 {
            let forward = to_camera.normalize();
            let up = Vec3::new(0.0, 1.0, 0.0);
            let right = up.cross(forward).normalize();
            let adjusted_up = forward.cross(right);

            transform.rotation = Quat::from_mat3(&Mat3::from_cols(right, adjusted_up, forward));
        }
    }
}

pub unsafe fn billboard_create_buffers(
    billboard: &mut BillboardData,
    backend: &mut VulkanBackend<'_>,
) -> Result<()> {
    backend.create_billboard_buffers(&mut billboard.mesh, &mut billboard.render_state)
}

pub fn billboard_transform_update_look_at(
    transform: &mut BillboardTransform,
    camera_position: Vec3,
    world_up: Vec3,
) {
    let forward = (camera_position - transform.position).normalize();
    let right = world_up.cross(forward).normalize();
    let up = forward.cross(right);

    let rotation = Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        forward.extend(0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    );

    let translation = Mat4::from_translation(transform.position);
    transform.model_matrix = translation * rotation;
}

pub fn billboard_transform_set_position(transform: &mut BillboardTransform, position: Vec3) {
    transform.position = position;
}
