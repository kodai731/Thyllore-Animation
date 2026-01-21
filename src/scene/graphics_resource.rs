use crate::animation::AnimationSystem;
use crate::render::ObjectUBO;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::vulkan::*;
use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};

pub use crate::vulkanr::descriptor::{
    FrameDescriptorSet, Material, MaterialId, MaterialManager, ObjectDescriptorSet, ObjectId,
};
pub use crate::vulkanr::resource::MeshBuffer;

#[derive(Clone, Debug)]
pub struct NodeData {
    pub index: usize,
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_transform: Matrix4<f32>,
    pub global_transform: Matrix4<f32>,
}

impl Default for NodeData {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            parent_index: None,
            local_transform: Matrix4::identity(),
            global_transform: Matrix4::identity(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GraphicsResources {
    pub frame_set: FrameDescriptorSet,
    pub materials: MaterialManager,
    pub objects: ObjectDescriptorSet,
    pub meshes: Vec<MeshBuffer>,
    pub mesh_material_ids: Vec<MaterialId>,
}

impl GraphicsResources {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
        max_materials: u32,
        max_objects: usize,
    ) -> anyhow::Result<Self> {
        let frame_set = FrameDescriptorSet::new(instance, rrdevice, swapchain_image_count)?;
        let materials = MaterialManager::new(rrdevice, max_materials)?;
        let objects =
            ObjectDescriptorSet::new(instance, rrdevice, swapchain_image_count, max_objects)?;

        Ok(Self {
            frame_set,
            materials,
            objects,
            meshes: Vec::new(),
            mesh_material_ids: Vec::new(),
        })
    }

    pub fn prepare_skinned_vertices(&mut self, animation: &AnimationSystem) -> Vec<usize> {
        let mut updated_mesh_ids = Vec::new();

        for mesh_idx in 0..self.meshes.len() {
            let (skin_data, skeleton_id) = {
                let mesh = &self.meshes[mesh_idx];
                (mesh.skin_data.clone(), mesh.skeleton_id)
            };

            let Some(skin_data) = skin_data else {
                continue;
            };
            let Some(skeleton_id) = skeleton_id else {
                continue;
            };
            let Some(skeleton) = animation.get_skeleton(skeleton_id) else {
                continue;
            };

            let vertex_count = skin_data.base_positions.len();
            let mut skinned_positions = vec![Vector3::new(0.0, 0.0, 0.0); vertex_count];
            let mut skinned_normals = vec![Vector3::new(0.0, 1.0, 0.0); vertex_count];

            skin_data.apply_skinning(skeleton, &mut skinned_positions, &mut skinned_normals);

            let mesh = &mut self.meshes[mesh_idx];
            for (i, pos) in skinned_positions.iter().enumerate() {
                if i < mesh.vertex_data.vertices.len() {
                    mesh.vertex_data.vertices[i].pos.x = pos.x;
                    mesh.vertex_data.vertices[i].pos.y = pos.y;
                    mesh.vertex_data.vertices[i].pos.z = pos.z;
                }
            }
            for (i, normal) in skinned_normals.iter().enumerate() {
                if i < mesh.vertex_data.vertices.len() {
                    mesh.vertex_data.vertices[i].normal.x = normal.x;
                    mesh.vertex_data.vertices[i].normal.y = normal.y;
                    mesh.vertex_data.vertices[i].normal.z = normal.z;
                }
            }

            updated_mesh_ids.push(mesh_idx);
        }

        updated_mesh_ids
    }

    pub fn prepare_node_animation(
        &mut self,
        nodes: &mut [NodeData],
        animation: &AnimationSystem,
        node_animation_scale: f32,
    ) -> Vec<usize> {
        static mut LOG_COUNT: u32 = 0;

        Self::compute_node_global_transforms(nodes, animation);

        let mut updated_mesh_indices = Vec::new();
        let scale = node_animation_scale;

        for (mesh_idx, mesh) in self.meshes.iter_mut().enumerate() {
            if mesh.skin_data.is_some() || mesh.base_vertices.is_empty() {
                continue;
            }

            let Some(node_idx) = mesh.node_index else {
                continue;
            };

            let node_found = nodes.iter().find(|n| n.index == node_idx);
            let Some(node) = node_found else {
                continue;
            };

            let transform = node.global_transform;

            for (i, v) in mesh.vertex_data.vertices.iter_mut().enumerate() {
                if i < mesh.base_vertices.len() {
                    let base = &mesh.base_vertices[i];
                    let pos = transform * Vector4::new(base.pos.x, base.pos.y, base.pos.z, 1.0);
                    v.pos.x = pos.x * scale;
                    v.pos.y = pos.y * scale;
                    v.pos.z = pos.z * scale;
                }
            }

            updated_mesh_indices.push(mesh_idx);
        }

        unsafe {
            if LOG_COUNT < 1 && !updated_mesh_indices.is_empty() {
                crate::log!(
                    "prepare_node_anim: updated {} meshes",
                    updated_mesh_indices.len()
                );
                LOG_COUNT += 1;
            }
        }

        updated_mesh_indices
    }

    pub unsafe fn update_objects(
        &self,
        rrdevice: &RRDevice,
        image_index: usize,
        model: Matrix4<f32>,
    ) -> anyhow::Result<()> {
        for mesh in &self.meshes {
            let object_ubo = ObjectUBO { model };
            self.objects
                .update(rrdevice, image_index, mesh.object_index, &object_ubo)?;
        }
        Ok(())
    }

    pub fn get_material_id(&self, mesh_index: usize) -> Option<MaterialId> {
        self.mesh_material_ids.get(mesh_index).copied()
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn get_layouts(&self) -> [vk::DescriptorSetLayout; 3] {
        [
            self.frame_set.layout,
            self.materials.layout,
            self.objects.layout,
        ]
    }

    pub fn get_layouts_without_material(&self) -> [vk::DescriptorSetLayout; 2] {
        [self.frame_set.layout, self.objects.layout]
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

        crate::log!("Model bounds: min=({:.2}, {:.2}, {:.2}), max=({:.2}, {:.2}, {:.2}), center=({:.2}, {:.2}, {:.2})",
            min.x, min.y, min.z, max.x, max.y, max.z, center.x, center.y, center.z);

        Some((min, max, center))
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        for mesh in &mut self.meshes {
            mesh.destroy(rrdevice);
        }
        self.meshes.clear();
        self.mesh_material_ids.clear();

        self.frame_set.destroy(&rrdevice.device);
        self.materials.destroy(&rrdevice.device);
        self.objects.destroy(&rrdevice.device);
    }

    pub unsafe fn clear_meshes(&mut self, rrdevice: &RRDevice) {
        for mesh in &mut self.meshes {
            mesh.destroy(rrdevice);
        }
        self.meshes.clear();
        self.mesh_material_ids.clear();
    }

    fn compute_node_global_transforms(nodes: &mut [NodeData], animation: &AnimationSystem) {
        static mut TRANSFORM_LOG: u32 = 0;

        if nodes.is_empty() {
            return;
        }

        let mut matched_count = 0;
        for skeleton in &animation.skeletons {
            for bone in &skeleton.bones {
                if let Some(node) = nodes.iter_mut().find(|n| n.name == bone.name) {
                    unsafe {
                        if TRANSFORM_LOG < 1 {
                            let orig = node.local_transform;
                            let anim = bone.local_transform;
                            let orig_scale = (
                                (orig[0][0] * orig[0][0]
                                    + orig[0][1] * orig[0][1]
                                    + orig[0][2] * orig[0][2])
                                    .sqrt(),
                                (orig[1][0] * orig[1][0]
                                    + orig[1][1] * orig[1][1]
                                    + orig[1][2] * orig[1][2])
                                    .sqrt(),
                                (orig[2][0] * orig[2][0]
                                    + orig[2][1] * orig[2][1]
                                    + orig[2][2] * orig[2][2])
                                    .sqrt(),
                            );
                            let anim_scale = (
                                (anim[0][0] * anim[0][0]
                                    + anim[0][1] * anim[0][1]
                                    + anim[0][2] * anim[0][2])
                                    .sqrt(),
                                (anim[1][0] * anim[1][0]
                                    + anim[1][1] * anim[1][1]
                                    + anim[1][2] * anim[1][2])
                                    .sqrt(),
                                (anim[2][0] * anim[2][0]
                                    + anim[2][1] * anim[2][1]
                                    + anim[2][2] * anim[2][2])
                                    .sqrt(),
                            );
                            crate::log!(
                                "  bone '{}' node[{}]: orig_t=[{:.2},{:.2},{:.2}] anim_t=[{:.2},{:.2},{:.2}]",
                                bone.name, node.index,
                                orig[3][0], orig[3][1], orig[3][2],
                                anim[3][0], anim[3][1], anim[3][2]
                            );
                            crate::log!(
                                "    orig_s=[{:.2},{:.2},{:.2}] anim_s=[{:.2},{:.2},{:.2}]",
                                orig_scale.0,
                                orig_scale.1,
                                orig_scale.2,
                                anim_scale.0,
                                anim_scale.1,
                                anim_scale.2
                            );
                        }
                    }
                    node.local_transform = bone.local_transform;
                    matched_count += 1;
                }
            }
        }

        unsafe {
            if TRANSFORM_LOG < 1 {
                crate::log!(
                    "compute_node_global_transforms: {} bones matched to {} nodes",
                    matched_count,
                    nodes.len()
                );
                crate::log!("=== Node Hierarchy (with transforms) ===");
                for node in nodes.iter() {
                    let parent_name = node
                        .parent_index
                        .and_then(|pi| nodes.iter().find(|pn| pn.index == pi))
                        .map(|pn| pn.name.as_str())
                        .unwrap_or("(root)");
                    let lt = node.local_transform;
                    let scale = (
                        (lt[0][0] * lt[0][0] + lt[0][1] * lt[0][1] + lt[0][2] * lt[0][2]).sqrt(),
                        (lt[1][0] * lt[1][0] + lt[1][1] * lt[1][1] + lt[1][2] * lt[1][2]).sqrt(),
                        (lt[2][0] * lt[2][0] + lt[2][1] * lt[2][1] + lt[2][2] * lt[2][2]).sqrt(),
                    );
                    if scale.0 > 1.01 || scale.1 > 1.01 || scale.2 > 1.01 {
                        crate::log!(
                            "  node[{}] '{}' SCALE=[{:.1},{:.1},{:.1}] parent='{}'",
                            node.index,
                            node.name,
                            scale.0,
                            scale.1,
                            scale.2,
                            parent_name
                        );
                    }
                }
                TRANSFORM_LOG += 1;
            }
        }

        let node_count = nodes.len();

        fn compute_global(
            nodes: &[NodeData],
            node_idx: usize,
            computed: &mut [bool],
            global_transforms: &mut [Matrix4<f32>],
        ) -> Matrix4<f32> {
            if computed[node_idx] {
                return global_transforms[node_idx];
            }

            let local = nodes[node_idx].local_transform;
            let global = if let Some(parent_idx) = nodes[node_idx].parent_index {
                if let Some(parent_array_idx) = nodes.iter().position(|n| n.index == parent_idx) {
                    let parent_global =
                        compute_global(nodes, parent_array_idx, computed, global_transforms);
                    parent_global * local
                } else {
                    local
                }
            } else {
                local
            };

            global_transforms[node_idx] = global;
            computed[node_idx] = true;
            global
        }

        let mut computed = vec![false; node_count];
        let mut global_transforms = vec![Matrix4::identity(); node_count];

        for i in 0..node_count {
            compute_global(nodes, i, &mut computed, &mut global_transforms);
        }

        for (i, node) in nodes.iter_mut().enumerate() {
            node.global_transform = global_transforms[i];
        }
    }

    pub fn create_pipeline_key(
        &self,
        vertex_shader: &str,
        fragment_shader: &str,
        topology: vk::PrimitiveTopology,
        polygon_mode: vk::PolygonMode,
        cull_mode: vk::CullModeFlags,
        depth_test_enable: bool,
        blend_enable: bool,
        render_pass: vk::RenderPass,
    ) -> crate::vulkanr::pipeline::PipelineKey {
        crate::vulkanr::pipeline::PipelineKey::new(
            vertex_shader,
            fragment_shader,
            topology,
            polygon_mode,
            cull_mode,
            depth_test_enable,
            depth_test_enable,
            blend_enable,
            vk::SampleCountFlags::_1,
            1,
            render_pass,
        )
    }
}
