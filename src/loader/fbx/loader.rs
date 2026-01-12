use crate::math::coordinate_system::fbx_to_world;
use crate::scene::animation::{
    AnimationClip, AnimationSystem, Keyframe, Skeleton, SkinData, TransformChannel,
};
use crate::vulkanr::data::{Vertex, VertexData};
use crate::math::{Vec2, Vec3, Vec4};
use anyhow::Result;
use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Vector4};
use std::collections::HashMap;

use super::fbx::{load_fbx_with_russimp, FbxModel};

#[derive(Clone, Debug)]
pub struct FbxNodeInfo {
    pub index: usize,
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_transform: Matrix4<f32>,
}

impl Default for FbxNodeInfo {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            parent_index: None,
            local_transform: Matrix4::identity(),
        }
    }
}

pub struct FbxMeshData {
    pub vertex_data: VertexData,
    pub skin_data: Option<SkinData>,
    pub skeleton_id: Option<u32>,
    pub texture_path: Option<String>,
    pub node_index: Option<usize>,
    pub local_vertices: Vec<Vertex>,
    pub base_positions: Vec<[f32; 3]>,
}

pub struct FbxLoadResult {
    pub meshes: Vec<FbxMeshData>,
    pub nodes: Vec<FbxNodeInfo>,
    pub animation_system: AnimationSystem,
    pub has_skinned_meshes: bool,
    pub has_armature: bool,
}

pub fn load_fbx_to_render_resources(path: &str) -> Result<FbxLoadResult> {
    let fbx_model = load_fbx_with_russimp(path)?;
    convert_fbx_model_to_render_resources(fbx_model)
}

fn convert_fbx_model_to_render_resources(fbx_model: FbxModel) -> Result<FbxLoadResult> {
    let mut animation_system = AnimationSystem::new();

    let has_armature = !fbx_model.nodes.is_empty();

    let skeleton_id = if has_armature {
        let skeleton = convert_nodes_to_skeleton(&fbx_model.nodes);
        Some(animation_system.add_skeleton(skeleton))
    } else {
        None
    };

    if let Some(skel_id) = skeleton_id {
        for fbx_data in &fbx_model.fbx_data {
            for cluster in &fbx_data.clusters {
                if let Some(skeleton) = animation_system.get_skeleton_mut(skel_id) {
                    if let Some(&bone_id) = skeleton.bone_name_to_id.get(&cluster.bone_name) {
                        if let Some(bone) = skeleton.get_bone_mut(bone_id) {
                            bone.inverse_bind_pose = cluster.inverse_bind_pose;
                        }
                    }
                }
            }
        }
    }

    for fbx_anim in &fbx_model.animations {
        let clip = convert_fbx_animation_to_clip(fbx_anim, &animation_system, skeleton_id);
        animation_system.add_clip(clip);
    }

    let nodes = convert_bone_nodes_to_node_info(&fbx_model.nodes);

    let mut meshes = Vec::new();
    let mut has_skinned_meshes = false;

    for fbx_data in &fbx_model.fbx_data {
        let mesh_data = convert_fbx_data_to_mesh(fbx_data, skeleton_id, &animation_system, &nodes);
        if mesh_data.skin_data.is_some() {
            has_skinned_meshes = true;
        }
        meshes.push(mesh_data);
    }

    log_fbx_scale_info(&meshes);

    if !animation_system.clips.is_empty() {
        animation_system.play(0);
    }

    Ok(FbxLoadResult {
        meshes,
        nodes,
        animation_system,
        has_skinned_meshes,
        has_armature,
    })
}

fn convert_bone_nodes_to_node_info(
    bone_nodes: &HashMap<String, super::fbx::BoneNode>,
) -> Vec<FbxNodeInfo> {
    let mut nodes = Vec::new();
    let mut name_to_index: HashMap<String, usize> = HashMap::new();

    let mut sorted_names: Vec<&String> = bone_nodes.keys().collect();
    sorted_names.sort();

    for (index, name) in sorted_names.iter().enumerate() {
        name_to_index.insert((*name).clone(), index);
    }

    for (index, name) in sorted_names.iter().enumerate() {
        if let Some(bone_node) = bone_nodes.get(*name) {
            let parent_index = bone_node
                .parent
                .as_ref()
                .and_then(|parent_name| name_to_index.get(parent_name).copied());

            let needs_coord_conversion = parent_index.is_none()
                || *name == "RootNode"
                || bone_node.parent.as_ref().map_or(false, |p| p == "RootNode");

            let local_transform = if needs_coord_conversion {
                fbx_to_world() * bone_node.local_transform
            } else {
                bone_node.local_transform
            };

            nodes.push(FbxNodeInfo {
                index,
                name: (*name).clone(),
                parent_index,
                local_transform,
            });
        }
    }

    nodes
}

fn log_fbx_scale_info(meshes: &[FbxMeshData]) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut min_z = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut max_z = f32::MIN;
    let mut total_vertices = 0;

    for mesh in meshes {
        for v in &mesh.vertex_data.vertices {
            min_x = min_x.min(v.pos.x);
            min_y = min_y.min(v.pos.y);
            min_z = min_z.min(v.pos.z);
            max_x = max_x.max(v.pos.x);
            max_y = max_y.max(v.pos.y);
            max_z = max_z.max(v.pos.z);
            total_vertices += 1;
        }
    }

    if total_vertices > 0 {
        let size_x = max_x - min_x;
        let size_y = max_y - min_y;
        let size_z = max_z - min_z;
        let max_dimension = size_x.max(size_y).max(size_z);

        crate::log!("=== FBX Scale Info (after unit conversion to meters) ===");
        crate::log!("  Total vertices: {}", total_vertices);
        crate::log!("  Bounding box min: ({:.4}, {:.4}, {:.4})", min_x, min_y, min_z);
        crate::log!("  Bounding box max: ({:.4}, {:.4}, {:.4})", max_x, max_y, max_z);
        crate::log!("  Size: ({:.4}, {:.4}, {:.4})", size_x, size_y, size_z);
        crate::log!("  Max dimension: {:.4} meters", max_dimension);
    }
}

fn convert_nodes_to_skeleton(nodes: &HashMap<String, super::fbx::BoneNode>) -> Skeleton {
    let mut skeleton = Skeleton::new("fbx_skeleton");

    let mut name_to_id: HashMap<String, u32> = HashMap::new();
    let mut sorted_names: Vec<&String> = nodes.keys().collect();
    sorted_names.sort();

    for name in &sorted_names {
        if let Some(node) = nodes.get(*name) {
            if node.parent.is_none() {
                add_bone_recursive(&mut skeleton, *name, nodes, &mut name_to_id);
            }
        }
    }

    for name in &sorted_names {
        if !name_to_id.contains_key(*name) {
            if let Some(_node) = nodes.get(*name) {
                add_bone_recursive(&mut skeleton, *name, nodes, &mut name_to_id);
            }
        }
    }

    skeleton
}

fn add_bone_recursive(
    skeleton: &mut Skeleton,
    name: &str,
    nodes: &HashMap<String, super::fbx::BoneNode>,
    name_to_id: &mut HashMap<String, u32>,
) -> u32 {
    if let Some(&id) = name_to_id.get(name) {
        return id;
    }

    let node = match nodes.get(name) {
        Some(n) => n,
        None => return 0,
    };

    let parent_id = if let Some(ref parent_name) = node.parent {
        if let Some(&pid) = name_to_id.get(parent_name) {
            Some(pid)
        } else {
            let pid = add_bone_recursive(skeleton, parent_name, nodes, name_to_id);
            Some(pid)
        }
    } else {
        None
    };

    let bone_id = skeleton.add_bone(name, parent_id);
    name_to_id.insert(name.to_string(), bone_id);

    if let Some(bone) = skeleton.get_bone_mut(bone_id) {
        let needs_coord_conversion = parent_id.is_none()
            || name == "RootNode"
            || node.parent.as_ref().map_or(false, |p| p == "RootNode");

        if needs_coord_conversion {
            bone.local_transform = fbx_to_world() * node.local_transform;
        } else {
            bone.local_transform = node.local_transform;
        }
    }

    for (child_name, child_node) in nodes {
        if let Some(ref parent) = child_node.parent {
            if parent == name && !name_to_id.contains_key(child_name) {
                add_bone_recursive(skeleton, child_name, nodes, name_to_id);
            }
        }
    }

    bone_id
}

fn convert_fbx_animation_to_clip(
    fbx_anim: &super::fbx::FbxAnimation,
    animation_system: &AnimationSystem,
    skeleton_id: Option<u32>,
) -> AnimationClip {
    let mut clip = AnimationClip::new(&fbx_anim.name);
    clip.duration = fbx_anim.duration;

    let skeleton = skeleton_id.and_then(|id| animation_system.get_skeleton(id));

    for (bone_name, bone_anim) in &fbx_anim.bone_animations {
        let bone_id = if let Some(skel) = skeleton {
            skel.bone_name_to_id.get(bone_name).copied()
        } else {
            None
        };

        if let Some(bid) = bone_id {
            let mut channel = TransformChannel::default();

            for key in &bone_anim.translation_keys {
                channel.translation.push(Keyframe {
                    time: key.time,
                    value: Vector3::new(key.value[0], key.value[1], key.value[2]),
                });
            }

            for key in &bone_anim.rotation_keys {
                channel.rotation.push(Keyframe {
                    time: key.time,
                    value: key.value,
                });
            }

            for key in &bone_anim.scale_keys {
                channel.scale.push(Keyframe {
                    time: key.time,
                    value: Vector3::new(key.value[0], key.value[1], key.value[2]),
                });
            }

            clip.add_channel(bid, channel);
        }
    }

    clip
}

fn convert_fbx_data_to_mesh(
    fbx_data: &super::fbx::FbxData,
    skeleton_id: Option<u32>,
    animation_system: &AnimationSystem,
    nodes: &[FbxNodeInfo],
) -> FbxMeshData {
    let mut vertices = Vec::with_capacity(fbx_data.positions.len());
    let mut local_vertices = Vec::with_capacity(fbx_data.positions.len());

    for i in 0..fbx_data.positions.len() {
        let pos = &fbx_data.positions[i];
        let local_pos = fbx_data.local_positions.get(i).unwrap_or(pos);
        let normal = fbx_data.normals.get(i).cloned().unwrap_or(Vector3::new(0.0, 1.0, 0.0));
        let tex_coord = fbx_data.tex_coords.get(i).cloned().unwrap_or([0.5, 0.5]);

        vertices.push(Vertex {
            pos: Vec3::new(pos.x, pos.y, pos.z),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            tex_coord: Vec2::new(tex_coord[0], tex_coord[1]),
            normal: Vec3::new(normal.x, normal.y, normal.z),
        });

        local_vertices.push(Vertex {
            pos: Vec3::new(local_pos.x, local_pos.y, local_pos.z),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            tex_coord: Vec2::new(tex_coord[0], tex_coord[1]),
            normal: Vec3::new(normal.x, normal.y, normal.z),
        });
    }

    let base_positions: Vec<[f32; 3]> = fbx_data
        .positions
        .iter()
        .map(|p| [p.x, p.y, p.z])
        .collect();

    let vertex_data = VertexData {
        vertices,
        indices: fbx_data.indices.clone(),
    };

    let skin_data = if !fbx_data.clusters.is_empty() && skeleton_id.is_some() {
        let skeleton = animation_system.get_skeleton(skeleton_id.unwrap());
        Some(convert_clusters_to_skin_data(fbx_data, skeleton_id.unwrap(), skeleton))
    } else {
        None
    };

    let node_index = fbx_data
        .parent_node
        .as_ref()
        .and_then(|parent_name| nodes.iter().find(|n| &n.name == parent_name).map(|n| n.index));

    FbxMeshData {
        vertex_data,
        skin_data,
        skeleton_id,
        texture_path: fbx_data.diffuse_texture.clone(),
        node_index,
        local_vertices,
        base_positions,
    }
}

fn convert_clusters_to_skin_data(
    fbx_data: &super::fbx::FbxData,
    skeleton_id: u32,
    skeleton: Option<&Skeleton>,
) -> SkinData {
    let vertex_count = fbx_data.positions.len();

    let mut bone_indices: Vec<Vector4<u32>> = vec![Vector4::new(0, 0, 0, 0); vertex_count];
    let mut bone_weights: Vec<Vector4<f32>> = vec![Vector4::new(0.0, 0.0, 0.0, 0.0); vertex_count];

    let mut vertex_bone_data: Vec<Vec<(u32, f32)>> = vec![Vec::new(); vertex_count];

    for cluster in fbx_data.clusters.iter() {
        let bone_idx = if let Some(skel) = skeleton {
            skel.bone_name_to_id.get(&cluster.bone_name).copied().unwrap_or(0)
        } else {
            0
        };

        for (i, &vert_idx) in cluster.vertex_indices.iter().enumerate() {
            if vert_idx < vertex_count {
                let weight = cluster.vertex_weights[i];
                if weight > 0.0 {
                    vertex_bone_data[vert_idx].push((bone_idx, weight));
                }
            }
        }
    }

    for (vert_idx, bones) in vertex_bone_data.iter().enumerate() {
        let mut sorted_bones = bones.clone();
        sorted_bones.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut indices = [0u32; 4];
        let mut weights = [0.0f32; 4];
        let mut total_weight = 0.0;

        for (i, &(bone_idx, weight)) in sorted_bones.iter().take(4).enumerate() {
            indices[i] = bone_idx;
            weights[i] = weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            for w in &mut weights {
                *w /= total_weight;
            }
        }

        bone_indices[vert_idx] = Vector4::new(indices[0], indices[1], indices[2], indices[3]);
        bone_weights[vert_idx] = Vector4::new(weights[0], weights[1], weights[2], weights[3]);
    }

    let base_positions: Vec<Vector3<f32>> = if !fbx_data.local_positions.is_empty() {
        fbx_data.local_positions.clone()
    } else {
        fbx_data.positions.clone()
    };

    let base_normals: Vec<Vector3<f32>> = if !fbx_data.local_normals.is_empty() {
        fbx_data.local_normals.clone()
    } else {
        fbx_data.normals.clone()
    };

    SkinData {
        skeleton_id,
        bone_indices,
        bone_weights,
        base_positions,
        base_normals,
    }
}
