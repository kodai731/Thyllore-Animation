use cgmath::SquareMatrix;

use crate::animation::{AnimationSystem, MorphAnimationSystem};
use crate::vulkanr::data::VertexData;

use super::{LoadedMesh, LoadedNode, ModelLoadResult, TextureData, TextureSource};

pub fn create_cube(size: f32, position: [f32; 3]) -> ModelLoadResult {
    let cube = crate::scene::CubeModel::new_at_position(size, position);

    let mesh = LoadedMesh {
        vertex_data: VertexData {
            vertices: cube.vertices.clone(),
            indices: cube.indices.clone(),
        },
        skin_data: None,
        skeleton_id: None,
        node_index: None,
        local_vertices: cube.vertices.clone(),
        texture: Some(TextureSource::Embedded(TextureData {
            data: vec![255u8, 255, 255, 255],
            width: 1,
            height: 1,
        })),
    };

    ModelLoadResult {
        meshes: vec![mesh],
        nodes: vec![LoadedNode {
            index: 0,
            name: "cube".to_string(),
            parent_index: None,
            local_transform: cgmath::Matrix4::identity(),
        }],
        animation_system: AnimationSystem::default(),
        morph_animation: MorphAnimationSystem::default(),
        has_skinned_meshes: false,
        node_animation_scale: 1.0,
    }
}
