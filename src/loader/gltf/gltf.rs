use crate::log;
use crate::math::*;
use anyhow::{anyhow, Result};
use cgmath::{Matrix4, Quaternion, Vector3, Vector4};
use core::result::Result::Ok;
use gltf::buffer::Data;
use gltf::{image, Document, Gltf, Node};
use std::collections::HashMap;
use std::fs::File;
use std::ptr::null;

#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub gltf_data: Vec<GltfData>,
    pub morph_animations: Vec<MorphAnimation>,
    pub joints: Vec<Joint>, // order by joint id
    pub joint_animations: Vec<Vec<JointAnimation>>,
    pub node_animations: Vec<NodeAnimation>, // Node transform animations
    pub node_joint_map: NodeJointMap,
    pub rrnodes: Vec<RRNode>,
    pub has_skinned_meshes: bool, // True if any mesh has JOINTS/WEIGHTS data
}

impl GltfModel {
    pub unsafe fn load_model(path: &str) -> Self {
        let mut gltf_model = GltfModel::default();
        gltf_model.morph_animations = Vec::new();
        load_gltf(&mut gltf_model, path);
        gltf_model
    }

    pub fn morph_target_index(&self, time: f32) -> usize {
        let end = self
            .morph_animations
            .last()
            .expect("morph_animations is empty");
        let start = self
            .morph_animations
            .first()
            .expect("morph_animations is empty");
        let mod_time = time % (end.key_frame - start.key_frame);
        let mut index = 0;
        // TODO: fix animation index
        let end_index = self.morph_animations.len();
        for i in 0..end_index {
            let morph_animation = &self.morph_animations[i];
            if mod_time <= morph_animation.key_frame {
                index = i;
                break;
            }
        }
        index
    }

    /// Update node animations and apply transformations to vertices
    pub unsafe fn update_node_animations(&mut self, time: f32) {
        if self.node_animations.is_empty() {
            return;
        }

        // Build a map of node index to animated transform
        let mut node_transforms: HashMap<usize, Matrix4<f32>> = HashMap::new();

        for node_anim in &self.node_animations {
            let transform = node_anim.get_transform_at_time(time);
            node_transforms.insert(node_anim.node_index, transform);
        }

        // Pre-calculate cumulative transforms for all nodes to avoid borrow issues
        let mut node_cumulative_transforms: HashMap<usize, Matrix4<f32>> = HashMap::new();
        for rrnode in &self.rrnodes {
            let node_index = rrnode.index as usize;
            // Calculate full cumulative transform including parent hierarchy
            let cumulative_transform = self.calculate_cumulative_transform(node_index, &node_transforms);
            node_cumulative_transforms.insert(node_index, cumulative_transform);
        }

        // Apply transforms to each gltf_data
        for gltf_data in self.gltf_data.iter_mut() {
            if gltf_data.vertices.is_empty() {
                continue;
            }

            // Find which node this mesh belongs to
            let node_id = gltf_data.vertices[0].node_id as usize;

            // Get cumulative transform for this node
            let cumulative_transform = node_cumulative_transforms
                .get(&node_id)
                .cloned()
                .unwrap_or(Matrix4::identity());

            // Apply transform to all vertices in this gltf_data
            for vertex in &mut gltf_data.vertices {
                // Get original local space position from animation_position
                let local_pos = vertex.animation_position;

                // Apply cumulative transform
                let pos_vec4 = cumulative_transform * Vector4::new(local_pos[0], local_pos[1], local_pos[2], 1.0);

                vertex.position = [pos_vec4.x, pos_vec4.y, pos_vec4.z];
            }
        }
    }

    /// Calculate cumulative transform for a node by traversing up the hierarchy
    fn calculate_cumulative_transform(
        &self,
        node_index: usize,
        node_transforms: &HashMap<usize, Matrix4<f32>>,
    ) -> Matrix4<f32> {
        // Find the node in rrnodes
        let rrnode = self.rrnodes.iter().find(|n| n.index as usize == node_index);

        if let Some(rrnode) = rrnode {
            // Get the base transform for this node
            let base_transform = if let Some(anim_transform) = node_transforms.get(&node_index) {
                // Use animated transform if available
                *anim_transform
            } else {
                // Use static transform from rrnode
                mat4_from_array(rrnode.transform)
            };

            // Find parent node by searching for a node that has this node as a child
            let parent_index = self.rrnodes.iter()
                .find(|parent| parent.children.contains(&(node_index as u16)))
                .map(|parent| parent.index as usize);

            if let Some(parent_idx) = parent_index {
                // Recursively calculate parent's cumulative transform
                let parent_transform = self.calculate_cumulative_transform(parent_idx, node_transforms);
                // Return parent_transform * local_transform
                parent_transform * base_transform
            } else {
                // This is a root node, return its own transform
                base_transform
            }
        } else {
            Matrix4::identity()
        }
    }

    pub fn set_joints(self: &mut Self, skin: &gltf::Skin, buffers: &Vec<Data>) {
        self.joints.clear();
        if self.node_joint_map.node_to_joint.len() <= 0 {
            self.node_joint_map.make_from_skin(skin);
        }

        let mut temp_joints: Vec<_> = skin.joints().collect();
        for (joint_index, node) in temp_joints.iter().enumerate() {
            let mut joint = Joint::default();
            joint.index = joint_index as u16;
            joint.name = node.name().unwrap().to_string();
            let joint_transform = mat4_from_array(node.transform().matrix());
            joint.transform = array_from_mat4(joint_transform);
            let node_index = self.node_joint_map.get_node_index(joint.index).unwrap();
            log!(
                "Joint Pushed: Node Index: {}, Node Name: {}, Joint Index: {}",
                node_index,
                joint.name,
                joint.index
            );
            self.joints.push(joint);
        }

        if let Some(_) = skin.inverse_bind_matrices() {
            let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
            if let Some(iter) = reader.read_inverse_bind_matrices() {
                log!("Inverse bind poses: {:?}", iter.len());
                for (i, mat) in iter.enumerate() {
                    let inverse_bind_pose_raw = mat4_from_array(mat);

                    // IMPORTANT: Do NOT scale inverse_bind_pose
                    // glTF files store inverse_bind_pose in the same coordinate system as vertices
                    // Both are already in meters, so no scaling is needed
                    let inverse_bind_pose = inverse_bind_pose_raw;

                    self.joints[i].inverse_bind_pose = array_from_mat4(inverse_bind_pose);
                    if i < 2 {
                        log!("Inverse bind pose {} (RAW from glTF file):", i);
                        log!("  rotation: [{:.6}, {:.6}, {:.6}], [{:.6}, {:.6}, {:.6}], [{:.6}, {:.6}, {:.6}]",
                             inverse_bind_pose.x.x, inverse_bind_pose.x.y, inverse_bind_pose.x.z,
                             inverse_bind_pose.y.x, inverse_bind_pose.y.y, inverse_bind_pose.y.z,
                             inverse_bind_pose.z.x, inverse_bind_pose.z.y, inverse_bind_pose.z.z);
                        log!("  translation: [{:.6}, {:.6}, {:.6}]",
                             inverse_bind_pose.w.x, inverse_bind_pose.w.y, inverse_bind_pose.w.z);
                    }
                }
            }
        }
    }

    pub fn apply_animation(self: &mut Self, time: f32, target_joint_id: usize, transform: Mat4) {
        // joints[0] = Root
        let joint = &self.joints[target_joint_id];
        let joint_animations = &self.joint_animations[target_joint_id];
        let mut joint_translate = Mat4::identity();
        let mut joint_rotation = Mat4::identity();
        let mut joint_scale = Mat4::identity();

        // Debug: Log for joints with animation data
        static mut LOG_COUNTER: u32 = 0;
        unsafe {
            LOG_COUNTER += 1;
            if LOG_COUNTER % 60 == 0 && joint_animations.len() > 0 {
                log!("apply_animation: time={:.4}, target_joint_id={}, animations_count={}",
                     time, target_joint_id, joint_animations.len());
            }
        }

        for (anim_idx, joint_animation) in joint_animations.iter().enumerate() {
            let key_frame_id = joint_animation.identify_key_frame_index(time);

            unsafe {
                if LOG_COUNTER % 60 == 0 && joint_animations.len() > 0 {
                    log!("  anim_idx={}, key_frame_id={}, keyframes_len={}, has_trans={}, has_rot={}, has_scale={}",
                         anim_idx,
                         key_frame_id,
                         joint_animation.key_frames.len(),
                         joint_animation.translations.len() > 0,
                         joint_animation.rotations.len() > 0,
                         joint_animation.scales.len() > 0);
                }
            }

            if joint_animation.scales.len() > key_frame_id {
                joint_scale = joint_animation.scales[key_frame_id] * joint_scale;
            }
            if joint_animation.rotations.len() > key_frame_id {
                joint_rotation = joint_animation.rotations[key_frame_id] * joint_rotation;
            }
            if joint_animation.translations.len() > key_frame_id {
                joint_translate = joint_animation.translations[key_frame_id] * joint_translate;
            }
        }

        let joint_inverse_bind_pose = mat4_from_array(joint.inverse_bind_pose);
        let joint_transform = transform * joint_translate * joint_rotation * joint_scale;

        // Debug: Log joint transforms and inverse bind pose for first few joints
        static mut LOGGED_JOINTS: [bool; 5] = [false; 5];
        unsafe {
            if LOG_COUNTER >= 60 && LOG_COUNTER <= 200 && target_joint_id < 5 && !LOGGED_JOINTS[target_joint_id] {
                LOGGED_JOINTS[target_joint_id] = true;
                log!("  joint_id={}, vertex_count={}, has_anim={}", target_joint_id, joint.vertex_indices.len(), joint_animations.len() > 0);
                log!("    joint_translate: {:?}", joint_translate);
                log!("    joint_inverse_bind_pose (first 2 rows): [{}, {}, {}, {}], [{}, {}, {}, {}]",
                     joint_inverse_bind_pose.x.x, joint_inverse_bind_pose.x.y, joint_inverse_bind_pose.x.z, joint_inverse_bind_pose.x.w,
                     joint_inverse_bind_pose.y.x, joint_inverse_bind_pose.y.y, joint_inverse_bind_pose.y.z, joint_inverse_bind_pose.y.w);
                log!("    joint_transform (first 2 rows): [{}, {}, {}, {}], [{}, {}, {}, {}]",
                     joint_transform.x.x, joint_transform.x.y, joint_transform.x.z, joint_transform.x.w,
                     joint_transform.y.x, joint_transform.y.y, joint_transform.y.z, joint_transform.y.w);
            }
        }

        for joint_vertex_id in &joint.vertex_indices {
            let vertex = &mut self.gltf_data[joint_vertex_id.gltf_data_index].vertices
                [joint_vertex_id.vertex_index];
            let weight = vertex.get_weight_from_joint_id(joint.index);

            // Skip if weight is zero
            if weight < 0.0001 {
                continue;
            }

            let mut pos = [
                vertex.position[0],
                vertex.position[1],
                vertex.position[2],
                1f32,
            ].to_vec4();

            // Apply skinning: joint_transform * inverse_bind_pose transforms from bind pose to current pose
            pos = joint_transform * joint_inverse_bind_pose * pos;
            pos = weight * pos;
            vertex.animation_position[0] += pos.x;
            vertex.animation_position[1] += pos.y;
            vertex.animation_position[2] += pos.z;
        }

        let child_indices = joint.child_joint_indices.clone();
        for child_index in child_indices {
            self.apply_animation(time, child_index as usize, joint_transform)
        }
    }

    // debug
    pub fn validate_vertex_position(self: &Self) {
        for (i, gltf_data) in self.gltf_data.iter().enumerate() {
            for vertex in &gltf_data.vertices {
                if !approx_equal_array3(&vertex.position, &vertex.animation_position) {
                    log!(
                        "invalid vertex animation position: joint id {:?}, gltf data index {}, vertex id {}",
                        vertex.joint_indices,
                        i,
                        vertex.index
                    );
                }
            }
        }
    }

    pub fn reset_vertices_animation_position(self: &mut Self, time: f32) {
        // For skeletal animation (meshes with JOINTS/WEIGHTS), reset to zero for weighted blending
        // For node animation (meshes without JOINTS/WEIGHTS), apply node transforms
        // Reset skinned meshes animation positions to zero for weighted sum of joint transformations
        self.gltf_data.iter_mut().for_each(|gltf_data| {
            if gltf_data.has_joints {
                // Skinned mesh: reset to zero because we'll accumulate weighted joint transformations
                gltf_data.vertices.iter_mut().for_each(|vertex| {
                    vertex.animation_position = [0.0, 0.0, 0.0];
                })
            }
        });

        // Apply node transforms to non-skinned meshes
        // This is called even if has_skinned_meshes is true, because the model may have both types
        if time < 0.1 {
            log!("=== Starting animation transform from node 0 with identity matrix ===");
        }
        let mut node_tree = Vec::default();
        node_tree.push(0 as u16);
        self.apply_node_transform(0, Matrix4::identity(), time, &mut node_tree);
    }

    /// Apply coordinate system fix to all animated vertices (after all transformations are complete)
    pub fn apply_coord_fix_to_animated_vertices(&mut self) {
        let fix_transform = fix_coord();
        for gltf_data in &mut self.gltf_data {
            // Apply to all vertices, both skinned and non-skinned
            for vertex in &mut gltf_data.vertices {
                let pos = [
                    vertex.animation_position[0],
                    vertex.animation_position[1],
                    vertex.animation_position[2],
                    1.0,
                ].to_vec4();
                let fixed_pos = fix_transform * pos;
                vertex.animation_position[0] = fixed_pos.x;
                vertex.animation_position[1] = fixed_pos.y;
                vertex.animation_position[2] = fixed_pos.z;
            }
        }
    }

    fn apply_node_transform(
        self: &mut Self,
        node_id: u16,
        transform: Mat4,
        time: f32,
        node_tree: &mut Vec<u16>,
    ) {
        if self.has_vertices(node_id) {
            let mut node_names = Vec::default();
            for node in node_tree.clone() {
                node_names.push(&self.rrnodes[node as usize].name);
            }
            log!("node tree {:?}, {:?}", node_tree, node_names);
        }
        let rrnode = &mut self.rrnodes[node_id as usize];

        // Check if this node has animation
        let mut rrnode_transform = mat4_from_array(rrnode.transform);
        for node_anim in &self.node_animations {
            if node_anim.node_index == node_id as usize {
                rrnode_transform = node_anim.get_transform_at_time(time);
                log!("Using animated transform for node {} at time {}", node_id, time);
                break;
            }
        }

        if rrnode.name.contains("Bone") {
            // rrnode_transform = Matrix4::identity();
        }
        if rrnode.name.contains("Joint") {
            // rrnode_transform = Matrix4::identity();
        }
        let node_transform = transform * rrnode_transform;

        // Debug: log transform calculation for key nodes in the hierarchy
        if (rrnode.index <= 10 || rrnode.index == 14 || rrnode.index == 15 || rrnode.index == 16 || rrnode.name.contains("NurbsPath.009_Material")) && time < 0.1 {
            // Calculate scale as column vector magnitudes (correct method for rotated matrices)
            let parent_scale_x = (transform.x.x * transform.x.x + transform.x.y * transform.x.y + transform.x.z * transform.x.z).sqrt();
            let parent_scale_y = (transform.y.x * transform.y.x + transform.y.y * transform.y.y + transform.y.z * transform.y.z).sqrt();
            let parent_scale_z = (transform.z.x * transform.z.x + transform.z.y * transform.z.y + transform.z.z * transform.z.z).sqrt();

            let local_scale_x = (rrnode_transform.x.x * rrnode_transform.x.x + rrnode_transform.x.y * rrnode_transform.x.y + rrnode_transform.x.z * rrnode_transform.x.z).sqrt();
            let local_scale_y = (rrnode_transform.y.x * rrnode_transform.y.x + rrnode_transform.y.y * rrnode_transform.y.y + rrnode_transform.y.z * rrnode_transform.y.z).sqrt();
            let local_scale_z = (rrnode_transform.z.x * rrnode_transform.z.x + rrnode_transform.z.y * rrnode_transform.z.y + rrnode_transform.z.z * rrnode_transform.z.z).sqrt();

            let result_scale_x = (node_transform.x.x * node_transform.x.x + node_transform.x.y * node_transform.x.y + node_transform.x.z * node_transform.x.z).sqrt();
            let result_scale_y = (node_transform.y.x * node_transform.y.x + node_transform.y.y * node_transform.y.y + node_transform.y.z * node_transform.y.z).sqrt();
            let result_scale_z = (node_transform.z.x * node_transform.z.x + node_transform.z.y * node_transform.z.y + node_transform.z.z * node_transform.z.z).sqrt();

            log!("ANIM CUMULATIVE: node={} ({}), parent_transform.translation={:?}, parent_scale={:?}",
                rrnode.index, rrnode.name,
                [transform.w.x, transform.w.y, transform.w.z],
                [parent_scale_x, parent_scale_y, parent_scale_z]);
            log!("ANIM CUMULATIVE: node={} ({}), local_transform.translation={:?}, local_scale={:?}",
                rrnode.index, rrnode.name,
                [rrnode_transform.w.x, rrnode_transform.w.y, rrnode_transform.w.z],
                [local_scale_x, local_scale_y, local_scale_z]);
            log!("ANIM CUMULATIVE: node={} ({}), result.translation={:?}, result_scale={:?}",
                rrnode.index, rrnode.name,
                [node_transform.w.x, node_transform.w.y, node_transform.w.z],
                [result_scale_x, result_scale_y, result_scale_z]);
        }

        if rrnode.vertex_indices.len() <= 0 && time < 1.0 {
            log!(
                "node transform {} {} {:?}",
                rrnode.index,
                rrnode.name,
                node_transform
            );
        } else if rrnode.vertex_indices.len() > 0 && time < 1.0 {
            log!(
                "node transform {} {} {:?}",
                rrnode.index,
                rrnode.name,
                node_transform
            );
        }
        for vertex_id in rrnode.vertex_indices.iter_mut() {
            let gltf_data_has_joints = self.gltf_data[vertex_id.gltf_data_index].has_joints;

            // Debug: log first vertex of NurbsPath.009 BEFORE skipping
            if rrnode.name.contains("NurbsPath.009") && vertex_id.vertex_index == 0 && time < 0.1 {
                log!("DEBUG BEFORE: node={}, has_joints={}, gltf_data_index={}",
                    rrnode.name, gltf_data_has_joints, vertex_id.gltf_data_index);
            }

            // Skip skinned mesh vertices (they use skeletal animation, not node animation)
            if gltf_data_has_joints {
                continue;
            }

            let vertex =
                &mut self.gltf_data[vertex_id.gltf_data_index].vertices[vertex_id.vertex_index];
            // Use original local space position (never changes)
            let mut position = [
                vertex.original_local_position[0],
                vertex.original_local_position[1],
                vertex.original_local_position[2],
                1f32,
            ].to_vec4();
            // Apply node transform (same as load time - no fix_coord)
            position = node_transform * position;

            vertex.animation_position = [position.x, position.y, position.z];

            // Debug: log first vertex of NurbsPath.009 AFTER transform
            if rrnode.name.contains("NurbsPath.009") && vertex_id.vertex_index == 0 && time < 0.1 {
                log!("DEBUG AFTER: original_local={:?}, after_node_transform={:?}, animation_position={:?}, load_position={:?}",
                    vertex.original_local_position, [position.x, position.y, position.z], vertex.animation_position, vertex.position);
            }
        }

        let children = rrnode.children.clone();
        for child in children {
            node_tree.push(child);
            self.apply_node_transform(child, node_transform, time, node_tree);
            node_tree.pop();
        }
    }

    fn has_vertices(self: &Self, node_index: u16) -> bool {
        self.rrnodes[node_index as usize].vertex_indices.len() > 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct GltfData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub image_indices: Vec<[u16; 4]>,
    pub image_data: Vec<ImageData>,
    pub morph_targets: Vec<MorphTarget>,
    pub has_joints: bool, // True if this mesh has JOINTS/WEIGHTS data (skinned mesh)
}

#[derive(Clone, Debug, Default)]
pub struct Vertex {
    pub index: usize,
    pub position: [f32; 3],
    pub animation_position: [f32; 3],
    pub original_local_position: [f32; 3], // Original local space position for node animation
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub tex_coord: [f32; 2],
    pub joint_indices: [u16; 4],
    pub joint_weights: [f32; 4],
    pub node_id: u16,
}

impl Vertex {
    pub fn identify_index_from_joint_id(&self, joint_id: u16) -> usize {
        for (i, vertex_joint_id) in self.joint_indices.iter().enumerate() {
            if *vertex_joint_id == joint_id {
                return i;
            }
        }
        log!(
            "invalid: this vertex {} is not included in joint {}",
            self.index,
            joint_id
        );
        3
    }

    pub fn get_weight_from_joint_id(&self, joint_id: u16) -> f32 {
        let index = self.identify_index_from_joint_id(joint_id);
        self.joint_weights[index]
    }
}

#[derive(Clone, Debug, Default)]
pub struct Joint {
    pub index: u16,
    pub name: String,
    pub vertex_indices: Vec<JointVertexIndex>,
    pub child_joint_indices: Vec<u16>,
    pub inverse_bind_pose: [[f32; 4]; 4],
    pub transform: [[f32; 4]; 4],
}

#[derive(Clone, Debug, Default)]
pub struct RRNode {
    pub index: u16,
    pub name: String,
    pub vertex_indices: Vec<JointVertexIndex>,
    pub transform: [[f32; 4]; 4],
    pub children: Vec<u16>,
}

#[derive(Clone, Debug, Default)]
pub struct JointVertexIndex {
    pub gltf_data_index: usize,
    pub vertex_index: usize,
}

impl PartialEq for JointVertexIndex {
    fn eq(&self, other: &Self) -> bool {
        self.gltf_data_index == other.gltf_data_index && self.vertex_index == other.vertex_index
    }
}

#[derive(Clone, Debug, Default)]
pub struct MorphTarget {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Default)]
pub struct MorphAnimation {
    pub key_frame: f32,
    pub weights: Vec<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct JointAnimation {
    pub key_frames: Vec<f32>,
    pub translations: Vec<Mat4>,
    pub rotations: Vec<Mat4>,
    pub scales: Vec<Mat4>,
}

impl JointAnimation {
    pub fn identify_key_frame_index(&self, time: f32) -> usize {
        let period = self.key_frames.last().unwrap();
        let time = time.rem_euclid(*period);
        for (i, key_frame) in self.key_frames.iter().enumerate() {
            if time < *key_frame {
                return i;
            }
        }
        return self.key_frames.len() - 1;
    }
}

#[derive(Clone, Debug)]
pub struct NodeAnimation {
    pub node_index: usize,
    pub translation_keyframes: Vec<f32>,
    pub translations: Vec<Vector3<f32>>,
    pub rotation_keyframes: Vec<f32>,
    pub rotations: Vec<cgmath::Quaternion<f32>>,
    pub scale_keyframes: Vec<f32>,
    pub scales: Vec<Vector3<f32>>,
    // Default transform from the node (used when property is not animated)
    pub default_translation: Vector3<f32>,
    pub default_rotation: cgmath::Quaternion<f32>,
    pub default_scale: Vector3<f32>,
}

impl Default for NodeAnimation {
    fn default() -> Self {
        NodeAnimation {
            node_index: 0,
            translation_keyframes: Vec::new(),
            translations: Vec::new(),
            rotation_keyframes: Vec::new(),
            rotations: Vec::new(),
            scale_keyframes: Vec::new(),
            scales: Vec::new(),
            default_translation: Vector3::new(0.0, 0.0, 0.0),
            default_rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
            default_scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl NodeAnimation {
    pub fn has_animation(&self) -> bool {
        !self.translation_keyframes.is_empty()
            || !self.rotation_keyframes.is_empty()
            || !self.scale_keyframes.is_empty()
    }

    pub fn get_transform_at_time(&self, time: f32) -> Matrix4<f32> {
        let translation = self.interpolate_translation(time);
        let rotation = self.interpolate_rotation(time);
        let scale = self.interpolate_scale(time);

        Matrix4::from_translation(translation)
            * Matrix4::from(rotation)
            * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
    }

    fn interpolate_translation(&self, time: f32) -> Vector3<f32> {
        if self.translations.is_empty() {
            return self.default_translation;  // Use node's default translation
        }
        if self.translations.len() == 1 {
            return self.translations[0];
        }

        let (idx0, idx1, t) = self.find_keyframe_indices(&self.translation_keyframes, time);
        let v0 = self.translations[idx0];
        let v1 = self.translations[idx1];
        v0 + (v1 - v0) * t
    }

    fn interpolate_rotation(&self, time: f32) -> cgmath::Quaternion<f32> {
        if self.rotations.is_empty() {
            return self.default_rotation;  // Use node's default rotation
        }
        if self.rotations.len() == 1 {
            return self.rotations[0];
        }

        let (idx0, idx1, t) = self.find_keyframe_indices(&self.rotation_keyframes, time);
        let q0 = self.rotations[idx0];
        let q1 = self.rotations[idx1];
        q0.nlerp(q1, t)
    }

    fn interpolate_scale(&self, time: f32) -> Vector3<f32> {
        if self.scales.is_empty() {
            return self.default_scale;  // Use node's default scale
        }
        if self.scales.len() == 1 {
            return self.scales[0];
        }

        let (idx0, idx1, t) = self.find_keyframe_indices(&self.scale_keyframes, time);
        let v0 = self.scales[idx0];
        let v1 = self.scales[idx1];
        v0 + (v1 - v0) * t
    }

    fn find_keyframe_indices(&self, keyframes: &Vec<f32>, time: f32) -> (usize, usize, f32) {
        if keyframes.is_empty() {
            return (0, 0, 0.0);
        }

        let period = keyframes.last().unwrap();
        let time = time.rem_euclid(*period);

        for i in 0..keyframes.len() - 1 {
            if time >= keyframes[i] && time < keyframes[i + 1] {
                let t = (time - keyframes[i]) / (keyframes[i + 1] - keyframes[i]);
                return (i, i + 1, t);
            }
        }

        (keyframes.len() - 1, keyframes.len() - 1, 0.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NodeJointMap {
    pub node_to_joint: HashMap<u16, u16>,
    pub joint_to_node: HashMap<u16, u16>,
}

impl NodeJointMap {
    pub fn make_from_skin(&mut self, skin: &gltf::Skin) {
        self.node_to_joint.clear();
        self.joint_to_node.clear();
        self.node_to_joint = HashMap::new();
        self.joint_to_node = HashMap::new();
        for (joint_index, joint_node) in skin.joints().enumerate() {
            self.node_to_joint
                .insert(joint_node.index() as u16, joint_index as u16);
            self.joint_to_node
                .insert(joint_index as u16, joint_node.index() as u16);
            log!(
                "Node Joint Map, node name: {}, node index: {}, joint: {}",
                joint_node.name().unwrap(),
                joint_node.index(),
                joint_index
            );
        }
    }

    pub fn get_joint_index(&self, node_index: u16) -> Option<u16> {
        match self.node_to_joint.get(&node_index) {
            Some(&joint_index) => Some(joint_index),
            None => {
                log!("Error: node {} is not in map", node_index);
                None
            }
        }
    }

    pub fn get_node_index(&self, joint_index: u16) -> Option<u16> {
        match self.joint_to_node.get(&joint_index) {
            Some(&node_index) => Some(node_index),
            None => {
                log!("Error: node {} is not in map", joint_index);
                None
            }
        }
    }

    pub fn contain_node_index(&self, node_index: u16) -> bool {
        self.node_to_joint.contains_key(&node_index)
    }
}

impl GltfData {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            image_indices: Vec::new(),
            image_data: Vec::new(),
            morph_targets: Vec::new(),
            has_joints: false,
        }
    }
}

impl MorphTarget {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            tangents: Vec::new(),
        }
    }
}

impl MorphAnimation {
    fn new() -> Self {
        Self {
            key_frame: 0.0,
            weights: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
}

unsafe fn load_gltf(gltf_model: &mut GltfModel, path: &str) {
    log!("Loading glTF file");
    let (gltf, buffers, images) = gltf::import(format!("{}", path)).expect("Failed to load model");

    log!("=== glTF File Structure ===");
    log!("Total skins: {}", gltf.skins().count());
    log!("Total nodes: {}", gltf.nodes().count());
    log!("Total meshes: {}", gltf.meshes().count());
    log!("Total animations: {}", gltf.animations().count());

    // Log skin information
    gltf.skins().enumerate().for_each(|(i, skin)| {
        log!("Skin {}: name={:?}, {} joints", i, skin.name(), skin.joints().count());
        log!("  Skeleton root: {:?}", skin.skeleton().map(|n| (n.index(), n.name())));
        gltf_model.node_joint_map.make_from_skin(&skin);
        gltf_model.set_joints(&skin, &buffers);
    });

    // Log which nodes reference which skin
    log!("=== Node-Skin Mappings ===");
    for node in gltf.nodes() {
        if let Some(skin) = node.skin() {
            log!("Node {} ({:?}) uses Skin {}", node.index(), node.name(), skin.index());
        }
    }

    // Log animation information
    log!("=== Animation Information ===");
    for (anim_idx, animation) in gltf.animations().enumerate() {
        log!("Animation {}: name={:?}, {} channels", anim_idx, animation.name(), animation.channels().count());
        for (chan_idx, channel) in animation.channels().enumerate() {
            let target = channel.target();
            let node = target.node();
            log!("  Channel {}: targets Node {} ({:?}), property={:?}",
                 chan_idx, node.index(), node.name(), target.property());
        }
    }

    for scene in gltf.scenes() {
        log!("=== Processing Scene {} ===", scene.index());
        let root_nodes: Vec<_> = scene.nodes().collect();
        log!("Scene root nodes: {:?}", root_nodes.iter().map(|n| (n.index(), n.name())).collect::<Vec<_>>());

        for node in root_nodes {
            log!("Processing root node {} ({:?})", node.index(), node.name());
            process_node(&gltf, &buffers, &images, &node, gltf_model, &Matrix4::identity(), None).unwrap();
        }
    }

    input_joint_vertex(gltf_model);
    log_node_hierarchy(gltf_model);
    validate_inverse_bind_pose(gltf_model, 0, fix_coord());
    load_white_texture_if_none(gltf_model);

    initialize_joint_animation(gltf_model);
    for animation in gltf.animations() {
        process_animation(&gltf, &buffers, animation, gltf_model).unwrap();
    }
    // validation
    for (i, joint_animation_joint) in gltf_model.joint_animations.iter().enumerate() {
        for (j, joint_animation_animation) in joint_animation_joint.iter().enumerate() {
            log!(
                "Joint Id {}, Animation Index {}, KeyFrameLength {}, TranslationLength {}, RotationLength {}, ScaleLength {}",
                i,
                j,
                joint_animation_animation.key_frames.len(),
                joint_animation_animation.translations.len(),
                joint_animation_animation.rotations.len(),
                joint_animation_animation.scales.len()
            );
        }
    }

    // Log animation summary
    log!("=== Animation Summary ===");
    log!("Has skinned meshes: {}", gltf_model.has_skinned_meshes);
    log!("Total node animations: {}", gltf_model.node_animations.len());
    for (i, node_anim) in gltf_model.node_animations.iter().enumerate() {
        log!("  Node animation {}: targets node {}, {} translation keyframes, {} rotation keyframes, {} scale keyframes",
             i, node_anim.node_index,
             node_anim.translation_keyframes.len(),
             node_anim.rotation_keyframes.len(),
             node_anim.scale_keyframes.len());
    }
    log!("Total joint animations: {}", gltf_model.joint_animations.len());
}

unsafe fn process_node(
    gltf: &Document,
    buffers: &Vec<Data>,
    images: &Vec<gltf::image::Data>,
    node: &Node,
    gltf_model: &mut GltfModel,
    parent_transform: &Matrix4<f32>,
    parent_node_index: Option<usize>,
) -> Result<()> {
    log!("Node {} {} (parent: {:?})", node.index(), node.name().unwrap(), parent_node_index);

    // Check if this node has a skin
    if let Some(skin) = node.skin() {
        log!("Node {} has skin: {} joints", node.index(), skin.joints().count());
    } else {
        log!("Node {} has NO skin", node.index());
    }

    let mut rrnode = RRNode::default();
    rrnode.index = node.index() as u16;
    log!(
        "node matrix {} {:?}",
        node.index(),
        node.transform().matrix()
    );
    let (node_translation, mut node_rotation, node_scale) =
        decompose(&mat4_from_array(node.transform().matrix()));

    // Debug: log Node 14's local transform and parent transform
    if node.index() == 14 {
        log!("Node 14 LOCAL (load time): translation={:?}, rotation={:?}, scale={:?}",
            [node_translation.x, node_translation.y, node_translation.z],
            node_rotation,
            [node_scale.x, node_scale.y, node_scale.z]);
        log!("Node 14 PARENT (load time): parent_transform={:?}",
            [parent_transform.w.x, parent_transform.w.y, parent_transform.w.z]);
    }

    // node_rotation = swap(&node_rotation);
    rrnode.transform = array_from_mat4(
        Matrix4::from_translation(node_translation)
            * Matrix4::from(node_rotation)
            * Matrix4::from_nonuniform_scale(node_scale[0], node_scale[1], node_scale[2]),
    );
    log!(
        "recompose node matrix {} {:?}",
        node.index(),
        rrnode.transform
    );
    rrnode.name = (*node.name().unwrap().to_string()).parse()?;
    for child in node.children() {
        rrnode.children.push(child.index() as u16);
    }

    // Calculate cumulative transform (parent * local) for initial pose
    let local_transform = mat4_from_array(rrnode.transform);
    let cumulative_transform = parent_transform * local_transform;

    // Debug: log cumulative transform for key nodes
    if node.index() <= 10 || node.index() == 14 || node.index() == 15 || node.index() == 16 || node.index() == 17 {
        let cum_scale_x = (cumulative_transform.x.x * cumulative_transform.x.x + cumulative_transform.x.y * cumulative_transform.x.y + cumulative_transform.x.z * cumulative_transform.x.z).sqrt();
        let cum_scale_y = (cumulative_transform.y.x * cumulative_transform.y.x + cumulative_transform.y.y * cumulative_transform.y.y + cumulative_transform.y.z * cumulative_transform.y.z).sqrt();
        let cum_scale_z = (cumulative_transform.z.x * cumulative_transform.z.x + cumulative_transform.z.y * cumulative_transform.z.y + cumulative_transform.z.z * cumulative_transform.z.z).sqrt();
        log!("LOAD CUMULATIVE: node={} ({:?}), cumulative.translation={:?}, cumulative.scale={:?}",
            node.index(), node.name(),
            [cumulative_transform.w.x, cumulative_transform.w.y, cumulative_transform.w.z],
            [cum_scale_x, cum_scale_y, cum_scale_z]);
    }

    // meshes
    if let Some(mesh) = node.mesh() {
        log!("mesh found");
        let primitives = mesh.primitives();
        let mut normals = Vec::new();

        // primitive
        primitives.for_each(|primitive| {
            log!("primitive found");
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            log!("Topology: {:?}", primitive.mode());

            // Log all attributes this primitive has
            log!("Primitive attributes:");
            for (semantic, _accessor) in primitive.attributes() {
                log!("  - {:?}", semantic);
            }

            // Check if this primitive has joints (skinned mesh)
            // Skinned meshes are transformed by joint matrices in the shader
            // Static meshes get the cumulative transform applied to vertices for initial pose
            let has_joints = reader.read_joints(0).is_some();
            log!("Primitive has joints: {}", has_joints);

            // Set flag if any mesh has skinned data
            if has_joints {
                gltf_model.has_skinned_meshes = true;
            }

            // Create a new GltfData for each primitive
            let mut gltf_data = GltfData::default();
            gltf_data.has_joints = has_joints;
            log!("Setting gltf_data.has_joints={} for mesh at node {}", has_joints, node.index());

            // texture
            if let Some(material) = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                let texture = material.texture();
                let image_index = texture.source().index();
                let image = &images[image_index];

                let size = (size_of::<u8>() * image.pixels.len()) as u64;
                let (width, height) = (image.width, image.height);
                let image_data = ImageData {
                    data: image.pixels.clone(),
                    size: size,
                    width: width,
                    height: height,
                };
                gltf_data.image_data.push(image_data);
            }

            // Add the new GltfData to the model
            gltf_model.gltf_data.push(gltf_data);

            let gltf_data_index = gltf_model.gltf_data.len() - 1;
            let gltf_data = gltf_model.gltf_data.last_mut().unwrap();

            let index_offset = gltf_data.vertices.len();
            if let Some(iter) = reader.read_indices() {
                log!("index count: {:?}", iter.clone().into_u32().len());
                for index in iter.into_u32() {
                    gltf_data
                        .indices
                        .push((index_offset + index as usize) as u32);
                }
            }

            if let Some(iter) = reader.read_positions() {
                log!("positions count {:?}", iter.len());
                for position in iter {
                    let mut vertex = Vertex::default();
                    vertex.index = gltf_data.vertices.len();

                    // Store original local space position permanently for node animation
                    vertex.original_local_position = position;

                    // Apply cumulative transform to static meshes for initial pose display
                    // Skinned meshes keep vertices in local space (transformed by joints in shader)
                    let transformed_position = if !has_joints {
                        // Transform vertex from local space to world space using cumulative transform
                        let pos_vec4 = cumulative_transform * Vector4::new(position[0], position[1], position[2], 1.0);
                        [pos_vec4.x, pos_vec4.y, pos_vec4.z]
                    } else {
                        // Keep skinned mesh vertices at original scale (mm units)
                        // inverse_bind_pose is now scaled by 0.001, so vertices should stay in mm
                        position
                    };

                    // Log first 5 vertices for debugging
                    if gltf_data.vertices.len() < 5 {
                        log!("VERTEX LOAD: node={:?}, idx={}, has_joints={}, original_pos=[{:.3}, {:.3}, {:.3}], transformed_pos=[{:.3}, {:.3}, {:.3}]",
                            node.name(), gltf_data.vertices.len(), has_joints,
                            position[0], position[1], position[2],
                            transformed_position[0], transformed_position[1], transformed_position[2]);
                    }

                    // Store position as-is without Y-flip
                    // Coordinate system conversion (Y-up to Z-up) will be handled by fix_coord()
                    vertex.position = transformed_position;
                    // Initialize animation_position with transformed position for non-animated meshes
                    vertex.animation_position = transformed_position;
                    vertex.node_id = node.index() as u16;

                    // Debug: log first vertex of NurbsPath.009_Material at load time
                    if node.name() == Some("NurbsPath.009_Material.001_0") && gltf_data.vertices.len() == 0 {
                        log!("LOAD TIME: node={}, node_index={}, original_local={:?}, cumulative_transform={:?}",
                            node.name().unwrap(), node.index(), position, cumulative_transform);
                        log!("LOAD TIME: transformed={:?}, has_joints={}", transformed_position, has_joints);
                    }

                    let mut vertex_id = JointVertexIndex::default();
                    vertex_id.gltf_data_index = gltf_data_index;
                    vertex_id.vertex_index = vertex.index;
                    rrnode.vertex_indices.push(vertex_id);

                    gltf_data.vertices.push(vertex);
                }
            }

            if let Some(gltf::mesh::util::ReadTexCoords::F32(gltf::accessor::Iter::Standard(
                                                                 iter,
                                                             ))) = reader.read_tex_coords(0)
            {
                for (i, texture_coord) in iter.enumerate() {
                    gltf_data.vertices[i].tex_coord = texture_coord;
                }
            }

            if let Some(iter) = reader.read_normals() {
                for normal in iter {
                    normals.push(normal);
                }
            }

            // joint
            if let Some(iter) = reader.read_joints(0) {
                let joints_vec: Vec<_> = iter.into_u16().collect();
                log!("Mesh {}: Found {} joint assignments", gltf_data_index, joints_vec.len());
                for (i, joint) in joints_vec.into_iter().enumerate() {
                    if i < 5 {  // Log only first 5 vertices
                        log!("  Vertex {}, Joint indices: {:?}", i, joint);
                    }
                    gltf_data.vertices[i].joint_indices = joint;
                }
            } else {
                log!("Mesh {}: NO joint data found (reader.read_joints returned None)", gltf_data_index);
            }

            if let Some(iter) = reader.read_weights(0) {
                let weights_vec: Vec<_> = iter.into_f32().collect();
                log!("Mesh {}: Found {} weight assignments", gltf_data_index, weights_vec.len());
                for (i, weight) in weights_vec.into_iter().enumerate() {
                    if i < 5 {  // Log only first 5 vertices
                        log!("  Vertex {}, Weights: {:?}", i, weight);
                    }
                    gltf_data.vertices[i].joint_weights = weight;
                }
            } else {
                log!("Mesh {}: NO weight data found (reader.read_weights returned None)", gltf_data_index);
            }

            // morph targets
            if let morph_targets = reader.read_morph_targets() {
                log!("morph targets count {:?}", morph_targets.len());
                for target in morph_targets {
                    let mut morph_target = MorphTarget::new();
                    let (positions, normals, tangents) = target;
                    // positions
                    if let Some(position_iter) = positions {
                        log!("morph positions count {:?}", position_iter.len());
                        for position in position_iter {
                            morph_target.positions.push(position);
                        }
                    }
                    // normals
                    if let Some(normal_iter) = normals {
                        log!("morph normals count {:?}", normal_iter.len());
                        for normal in normal_iter {
                            morph_target.normals.push(normal);
                        }
                    }
                    // tangents
                    if let Some(tangent_iter) = tangents {
                        log!("morph tangents count {:?}", tangent_iter.len());
                        for tangent in tangent_iter {
                            morph_target.tangents.push(tangent);
                        }
                    }
                    gltf_data.morph_targets.push(morph_target);
                }
            }

            // validate
            log!("vertex count {}", gltf_data.vertices.len());
            log!("indices count {}", gltf_data.indices.len());
            log!("morph targets count {}", gltf_data.morph_targets.len());
        });
    }

    gltf_model.rrnodes.push(rrnode);

    if gltf_model
        .node_joint_map
        .contain_node_index(node.index() as u16)
    {
        let joint_index = gltf_model
            .node_joint_map
            .get_joint_index(node.index() as u16)
            .unwrap();
        let joint = &mut gltf_model.joints[joint_index as usize];

        for child in node.children() {
            if gltf_model
                .node_joint_map
                .contain_node_index(child.index() as u16)
            {
                joint.child_joint_indices.push(
                    gltf_model
                        .node_joint_map
                        .get_joint_index(child.index() as u16)
                        .unwrap(),
                );
            }
        }
    }

    let children = node.children().clone();
    let mut children_ids = Vec::default();
    let mut children_names = Vec::new();
    for child in children {
        children_ids.push(child.index());
        children_names.push(child.name().unwrap().to_string());
    }
    log!(
        "node {} {} children: {:?} {:?}",
        node.index(),
        node.name().unwrap(),
        children_ids,
        children_names
    );

    for child in node.children() {
        process_node(gltf, buffers, images, &child, gltf_model, &cumulative_transform, Some(node.index()))?;
    }

    Ok(())
}

fn load_white_texture_if_none(gltf_model: &mut GltfModel) {
    for gltf_data in &mut gltf_model.gltf_data {
        if gltf_data.image_data.len() == 0 {
            let image_data = load_white_texture().unwrap();
            gltf_data.image_data.push(image_data);
        }
    }
}

// TODO: use vulkanr::image
fn load_white_texture() -> Result<ImageData> {
    let image = File::open("assets/textures/white.png")?;
    let decoder = png::Decoder::new(image);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut pixels)?;
    let size = reader.info().raw_bytes() as u64;
    let (width, height) = reader.info().size();
    Ok(ImageData {
        data: pixels,
        size,
        width,
        height,
    })
}

unsafe fn input_joint_vertex(gltf_model: &mut GltfModel) {
    log!("input_joint_vertex: processing {} meshes", gltf_model.gltf_data.len());
    let mut vertex_count = 0;
    for (i, gltf_data) in &mut gltf_model.gltf_data.iter().enumerate() {
        if gltf_data.vertices.len() <= 0 {
            log!("  mesh {}: no vertices, skipping", i);
            continue; // Changed from return to continue
        }
        if gltf_data.vertices[0].joint_indices.len() <= 0 {
            log!("  mesh {}: no joint indices, skipping", i);
            continue; // Changed from return to continue
        }

        log!("  mesh {}: {} vertices with joints", i, gltf_data.vertices.len());
        vertex_count += gltf_data.vertices.len();
        gltf_data.vertices.iter().for_each(|vertex| {
            // Only register vertex to joints with non-zero weights
            for (joint_idx, joint_index) in vertex.joint_indices.iter().enumerate() {
                let weight = vertex.joint_weights[joint_idx];
                // Skip joints with zero or negligible weight
                if weight < 0.0001 {
                    continue;
                }
                let mut joint_vertex_index = JointVertexIndex::default();
                joint_vertex_index.gltf_data_index = i;
                joint_vertex_index.vertex_index = vertex.index;
                if !gltf_model.joints[*joint_index as usize]
                    .vertex_indices
                    .contains(&joint_vertex_index)
                {
                    gltf_model.joints[*joint_index as usize]
                        .vertex_indices
                        .push(joint_vertex_index);
                }
            }
        });

        // validate
        for vertex in &gltf_data.vertices {
            let joint_indices = vertex.joint_indices;
            let mut is_vertex_found = false;
            for (j, joint_index) in joint_indices.iter().enumerate() {
                let target_joint = &gltf_model.joints[*joint_index as usize];
                let mut is_joint_found = false;
                for joint_vertex_index in &target_joint.vertex_indices {
                    if joint_vertex_index.vertex_index == vertex.index {
                        is_joint_found = true;
                        is_vertex_found = true;
                        break;
                    }
                }
                if !is_joint_found {
                    log!(
                        "invalid: joint index not found: Gltf Index {}, Joint Index Id {}, Joint Index {}",
                        i,
                        j,
                        joint_index
                    );
                }
            }
            if !is_vertex_found {
                log!("invalid: vertex {} is not included in Joint", vertex.index);
            }
        }
    }

    // rrnode
    let mut node_vertex_count = 0;
    for rrnode in &gltf_model.rrnodes {
        log!(
            "node {} {} vertices count: {}",
            rrnode.index,
            rrnode.name,
            rrnode.vertex_indices.len()
        );
        node_vertex_count += rrnode.vertex_indices.len();
    }
    log!(
        "total node vertices {} vertices {}",
        node_vertex_count,
        vertex_count,
    );

    // Log joint vertex counts
    log!("Joint vertex assignments:");
    for (i, joint) in gltf_model.joints.iter().enumerate() {
        log!("  joint {}: {} ({} vertices)", i, joint.name, joint.vertex_indices.len());
    }
}

unsafe fn log_node_hierarchy(gltf_model: &GltfModel) {
    for (i, joint) in gltf_model.joints.iter().enumerate() {
        let node_index = gltf_model
            .node_joint_map
            .get_node_index(joint.index)
            .unwrap();
        log!(
            "joint name: {}, node Id: {}, joint Id: {}, child joint indices: {:?}",
            joint.name,
            node_index,
            joint.index,
            joint.child_joint_indices
        );
    }
}

// debug test
unsafe fn validate_inverse_bind_pose(gltf_model: &GltfModel, joint_index: u16, transform: Mat4) {
    let joint = &gltf_model.joints[joint_index as usize];
    let inverse_bind_pose = mat4_from_array(joint.inverse_bind_pose);
    let joint_transform = mat4_from_array(joint.transform);
    log!("joint transform {}: {:?}", joint_index, joint_transform);
    let transform = transform * joint_transform;
    let multiplied = transform * inverse_bind_pose;
    if !approx_equal_mat4(&multiplied, &Mat4::identity()) {
        log!(
            "invalid: inverse transform is not invertible, joint id {}, multi product {:?}",
            joint_index,
            multiplied,
        );
    }
    for child in &joint.child_joint_indices {
        validate_inverse_bind_pose(gltf_model, *child, transform);
    }
}

unsafe fn initialize_joint_animation(gltf_model: &mut GltfModel) {
    for _ in 0..gltf_model.joints.len() {
        gltf_model.joint_animations.push(Vec::default())
    }
}

unsafe fn process_animation(
    gltf: &Document,
    buffers: &Vec<Data>,
    animation: gltf::Animation,
    gltf_model: &mut GltfModel,
) -> Result<()> {
    for channel in animation.channels() {
        let gltf_data_index = channel.animation().index();
        let gltf_data = &gltf_model.gltf_data[gltf_data_index];
        let target = channel.target();
        let node = target.node();
        log!("target animation index {}", target.animation().index());
        log!("node index {}", node.index());
        log!("node name {}", node.name().unwrap());
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let mut key_frames = Vec::new();
        let mut weights = Vec::new();
        let mut joint_translations = Vec::new();
        let mut node_translations = Vec::new(); // Store unscaled translations for node animations
        let mut joint_rotations = Vec::new();
        let mut joint_rotation_quats = Vec::new(); // Store quaternions for node animations
        let mut joint_scales = Vec::new();
        if let Some(inputs) = reader.read_inputs() {
            log!("KeyFrame Count: {:?}", inputs.len());
            for (i, input) in inputs.enumerate() {
                log!("KeyFrame input {}: {:?}", i, input);
                key_frames.push(input);
            }
        }

        if let Some(outputs) = reader.read_outputs() {
            use gltf::animation::util::ReadOutputs;
            match outputs {
                ReadOutputs::Translations(translations) => {
                    for (i, translation) in translations.enumerate() {
                        log!("Translation {}: {:?}", i, translation);
                        // Store translation for node animations
                        let node_translation = translation.to_vec3().into();
                        node_translations.push(node_translation);

                        // IMPORTANT: Do NOT scale joint animation translations
                        // glTF files store animation data in the same coordinate system as vertices
                        // Both are already in meters, so no scaling is needed
                        let joint_translation = translation.to_vec3();
                        let joint_translation_mat = Mat4::from_translation(joint_translation.into());
                        joint_translations.push(joint_translation_mat);
                        log!("Translation Matrix {}: {:?}", i, joint_translation_mat);
                    }
                }
                ReadOutputs::Rotations(rotations) => {
                    for (i, rotation) in rotations.into_f32().enumerate() {
                        log!("Rotation {}: {:?}", i, rotation);
                        // glTF quaternions are [x, y, z, w], cgmath Quaternion::new expects (w, x, y, z)
                        let quat = Quaternion::new(rotation[3], rotation[0], rotation[1], rotation[2])
                            .normalize();
                        joint_rotation_quats.push(quat);

                        // Also store as matrix for joint animations
                        let joint_rotaion_mat = Mat4::from(quat);
                        joint_rotations.push(joint_rotaion_mat);
                    }
                }
                ReadOutputs::Scales(scales) => {
                    for (i, scale) in scales.enumerate() {
                        log!("Scale {}: {:?}", i, scale);
                        let joint_scale_mat =
                            Mat4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
                        log!("Scale Matrix {} : {:?}", i, joint_scale_mat);
                        joint_scales.push(joint_scale_mat);
                    }
                }
                ReadOutputs::MorphTargetWeights(morph_target_weights) => {
                    let mut weight = Vec::new();
                    // TODO: multi data
                    let morph_target_length = gltf_data.morph_targets.len();
                    for (i, morph_target_weight) in morph_target_weights.into_f32().enumerate() {
                        log!("Morph Target Weight: {} {:?}", i, morph_target_weight);
                        weight.push(morph_target_weight);
                        if weight.len() >= morph_target_length {
                            weights.push(weight);
                            weight = Vec::new();
                        }
                    }
                }
            }
        }

        if key_frames.len() != weights.len() {
            log!("KeyFrame Count != Weight Count");
        }

        if key_frames.len() != 0 && weights.len() != 0 && key_frames.len() == weights.len() {
            for i in 0..key_frames.len() {
                let mut morph_animation = MorphAnimation::new();
                morph_animation.key_frame = key_frames[i];
                morph_animation.weights = weights[i].clone();
                gltf_model.morph_animations.push(morph_animation);
            }
        }

        if key_frames.len() > 0
            && (joint_translations.len() > 0
            || joint_rotations.len() > 0
            || joint_scales.len() > 0)
        {
            // Check if this node is a joint AND we have skinned meshes
            // If no skinned meshes, treat all animations as node animations
            let is_joint_node = gltf_model
                .node_joint_map
                .get_joint_index(node.index() as u16)
                .is_some();

            if is_joint_node && gltf_model.has_skinned_meshes {
                // This is a joint animation for skinned mesh
                let joint_id = gltf_model
                    .node_joint_map
                    .get_joint_index(node.index() as u16)
                    .unwrap();
                let mut joint_animation = JointAnimation::default();
                for i in 0..key_frames.len() {
                    joint_animation.key_frames.push(key_frames[i]);
                    if joint_translations.len() > 0 {
                        joint_animation.translations.push(joint_translations[i]);
                    }
                    if joint_rotations.len() > 0 {
                        joint_animation.rotations.push(joint_rotations[i]);
                    }
                    if joint_scales.len() > 0 {
                        joint_animation.scales.push(joint_scales[i]);
                    }
                }
                gltf_model.joint_animations[joint_id as usize].push(joint_animation);
            } else {
                // This is a node animation (non-joint)
                let mut node_animation = NodeAnimation::default();
                node_animation.node_index = node.index();

                // Store translation keyframes and values
                if node_translations.len() > 0 {
                    for i in 0..key_frames.len() {
                        node_animation.translation_keyframes.push(key_frames[i]);
                        // Use unscaled translation for node animations
                        node_animation.translations.push(node_translations[i]);
                    }
                }

                // Store rotation keyframes and values
                if joint_rotation_quats.len() > 0 {
                    for i in 0..key_frames.len() {
                        node_animation.rotation_keyframes.push(key_frames[i]);
                        // Use the quaternion directly from glTF data
                        node_animation.rotations.push(joint_rotation_quats[i]);
                    }
                }

                // Store scale keyframes and values
                if joint_scales.len() > 0 {
                    for i in 0..key_frames.len() {
                        node_animation.scale_keyframes.push(key_frames[i]);
                        // Extract scale from matrix
                        let mat = joint_scales[i];
                        let scale = Vector3::new(mat[0][0], mat[1][1], mat[2][2]);
                        node_animation.scales.push(scale);
                    }
                }

                // Set default transform from node (for properties not animated)
                let (default_trans, default_rot, default_scale) =
                    decompose(&mat4_from_array(node.transform().matrix()));
                node_animation.default_translation = default_trans;
                node_animation.default_rotation = default_rot;
                node_animation.default_scale = default_scale;

                log!("Node Animation created for node {}: {} keyframes", node.index(), key_frames.len());
                log!("  Default translation: {:?}", default_trans);
                gltf_model.node_animations.push(node_animation);
            }
        }

        // validate
        for i in 0..gltf_model.morph_animations.len() {
            for j in 0..gltf_model.morph_animations[i].weights.len() {
                let morph_animation = &gltf_model.morph_animations[i];
                log!(
                    "Morph Animation {} KeyFrame {:?} Weight {} {:?}",
                    i,
                    morph_animation.key_frame,
                    j,
                    morph_animation.weights[j]
                );
            }
        }
        log!("vertex count {:?}", gltf_data.vertices.len());
        if gltf_data.morph_targets.len() > 0 {
            // morphing
            log!(
                "target0 position count {:?}",
                gltf_data.morph_targets[0].positions.len()
            );
            log!(
                "morph animation0 weights count {:?}",
                gltf_model.morph_animations[0].weights.len()
            );
        }
        log!("key frame count {:?}", key_frames.len());
        log!("joint translation count {:?}", joint_translations.len());
        log!("joint rotation count {:?}", joint_rotations.len());
        log!("joint scale count {:?}", joint_scales.len());
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_default() {
        let vertex = Vertex::default();
        assert_eq!(vertex.position, [0.0, 0.0, 0.0]);
        assert_eq!(vertex.normal, [0.0, 0.0, 0.0]);
        assert_eq!(vertex.tex_coord, [0.0, 0.0]);
        assert_eq!(vertex.joint_indices, [0, 0, 0, 0]);
        assert_eq!(vertex.joint_weights, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_gltf_data_default() {
        let data = GltfData::default();
        assert_eq!(data.vertices.len(), 0);
        assert_eq!(data.indices.len(), 0);
        assert_eq!(data.morph_targets.len(), 0);
        assert_eq!(data.has_joints, false);
    }

    #[test]
    fn test_gltf_model_default() {
        let model = GltfModel::default();
        assert_eq!(model.gltf_data.len(), 0);
        assert_eq!(model.morph_animations.len(), 0);
        assert_eq!(model.joints.len(), 0);
        assert_eq!(model.has_skinned_meshes, false);
    }

    #[test]
    fn test_image_data_default() {
        let image = ImageData::default();
        assert_eq!(image.data.len(), 0);
        assert_eq!(image.size, 0);
        assert_eq!(image.width, 0);
        assert_eq!(image.height, 0);
    }

    #[test]
    fn test_vertex_position_modification() {
        let mut vertex = Vertex::default();
        vertex.position = [1.0, 2.0, 3.0];
        assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_vertex_normal_modification() {
        let mut vertex = Vertex::default();
        vertex.normal = [0.0, 1.0, 0.0];
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_vertex_tex_coord_modification() {
        let mut vertex = Vertex::default();
        vertex.tex_coord = [0.5, 0.5];
        assert_eq!(vertex.tex_coord, [0.5, 0.5]);
    }

    #[test]
    fn test_vertex_joint_data() {
        let mut vertex = Vertex::default();
        vertex.joint_indices = [0, 1, 2, 3];
        vertex.joint_weights = [0.4, 0.3, 0.2, 0.1];
        
        assert_eq!(vertex.joint_indices, [0, 1, 2, 3]);
        assert_eq!(vertex.joint_weights, [0.4, 0.3, 0.2, 0.1]);
    }

    #[test]
    fn test_joint_weights_sum() {
        let mut vertex = Vertex::default();
        vertex.joint_weights = [0.25, 0.25, 0.25, 0.25];
        
        let sum: f32 = vertex.joint_weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_morph_target_default() {
        let morph = MorphTarget::default();
        assert_eq!(morph.positions.len(), 0);
        assert_eq!(morph.normals.len(), 0);
        assert_eq!(morph.tangents.len(), 0);
    }

    #[test]
    fn test_joint_default() {
        let joint = Joint::default();
        assert_eq!(joint.index, 0);
        assert_eq!(joint.name, "");
        assert_eq!(joint.vertex_indices.len(), 0);
        assert_eq!(joint.child_joint_indices.len(), 0);
    }

    #[test]
    fn test_gltf_data_push_vertex() {
        let mut data = GltfData::default();
        let vertex = Vertex {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        data.vertices.push(vertex);
        
        assert_eq!(data.vertices.len(), 1);
        assert_eq!(data.vertices[0].position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_gltf_data_push_index() {
        let mut data = GltfData::default();
        data.indices.push(0);
        data.indices.push(1);
        data.indices.push(2);
        
        assert_eq!(data.indices.len(), 3);
        assert_eq!(data.indices, vec![0, 1, 2]);
    }
}

