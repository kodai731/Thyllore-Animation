use crate::animation::{AnimationSystem, MorphAnimationSystem};
use crate::scene::graphics_resource::{
    FrameDescriptorSet, MaterialId, MaterialManager, MeshBuffer, NodeData, ObjectDescriptorSet,
};
use cgmath::Vector3;

#[derive(Clone, Debug, Default)]
pub struct GpuDescriptors {
    pub frame_set: FrameDescriptorSet,
    pub objects: ObjectDescriptorSet,
}

impl GpuDescriptors {
    pub fn new(frame_set: FrameDescriptorSet, objects: ObjectDescriptorSet) -> Self {
        Self { frame_set, objects }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MaterialRegistry {
    pub materials: MaterialManager,
    pub mesh_material_ids: Vec<MaterialId>,
}

impl MaterialRegistry {
    pub fn new(materials: MaterialManager) -> Self {
        Self {
            materials,
            mesh_material_ids: Vec::new(),
        }
    }

    pub fn get_material_id(&self, mesh_index: usize) -> Option<MaterialId> {
        self.mesh_material_ids.get(mesh_index).copied()
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnimationRegistry {
    pub animation: AnimationSystem,
    pub morph_animation: MorphAnimationSystem,
}

impl AnimationRegistry {
    pub fn new() -> Self {
        Self {
            animation: AnimationSystem::new(),
            morph_animation: MorphAnimationSystem::new(),
        }
    }

    pub fn clear(&mut self) {
        self.animation.clear();
        self.morph_animation = MorphAnimationSystem::new();
    }
}

#[derive(Clone, Debug)]
pub struct ModelState {
    pub has_skinned_meshes: bool,
    pub node_animation_scale: f32,
}

impl Default for ModelState {
    fn default() -> Self {
        Self {
            has_skinned_meshes: false,
            node_animation_scale: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MeshAssets {
    pub meshes: Vec<MeshBuffer>,
}

impl MeshAssets {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn calculate_model_bounds(&self) -> Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)> {
        if self.meshes.is_empty() {
            return None;
        }

        let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
        let mut has_vertices = false;

        for mesh in &self.meshes {
            for vertex in &mesh.vertex_data.vertices {
                has_vertices = true;
                min.x = min.x.min(vertex.pos.x);
                min.y = min.y.min(vertex.pos.y);
                min.z = min.z.min(vertex.pos.z);
                max.x = max.x.max(vertex.pos.x);
                max.y = max.y.max(vertex.pos.y);
                max.z = max.z.max(vertex.pos.z);
            }
        }

        if !has_vertices {
            return None;
        }

        let center = Vector3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            (min.z + max.z) * 0.5,
        );

        Some((min, max, center))
    }
}

#[derive(Clone, Debug, Default)]
pub struct NodeAssets {
    pub nodes: Vec<NodeData>,
}

impl NodeAssets {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}
