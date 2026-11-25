use glam::{Mat4, Vec3};
use thyllore_model_core::MeshMorph;

#[derive(Clone, Debug)]
pub struct ClusterInfo {
    pub bone_name: String,
    pub transform: Mat4,
    pub transform_link: Mat4,
    pub inverse_bind_pose: Mat4,
    pub vertex_indices: Vec<usize>,
    pub vertex_weights: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct MeshPart {
    pub mesh_name: String,
    pub local_positions: Vec<Vec3>,
    pub parent_bone: Option<String>,
    pub local_transform: Mat4,
    pub vertex_offset: usize,
    pub vertex_count: usize,
}

#[derive(Clone, Debug)]
pub struct FbxData {
    pub positions: Vec<Vec3>,
    pub local_positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub local_normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub tex_coords: Vec<[f32; 2]>,
    pub clusters: Vec<ClusterInfo>,
    pub morph: MeshMorph,
    pub mesh_parts: Vec<MeshPart>,
    pub parent_node: Option<String>,
    pub mesh_node_name: Option<String>,
    pub material_name: Option<String>,
    pub diffuse_texture: Option<String>,
    pub diffuse_color: [f32; 3],
}

impl FbxData {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            local_positions: Vec::new(),
            normals: Vec::new(),
            local_normals: Vec::new(),
            indices: Vec::new(),
            tex_coords: Vec::new(),
            clusters: Vec::new(),
            morph: MeshMorph::default(),
            mesh_parts: Vec::new(),
            parent_node: None,
            mesh_node_name: None,
            material_name: None,
            diffuse_texture: None,
            diffuse_color: [0.8, 0.8, 0.8],
        }
    }
}
