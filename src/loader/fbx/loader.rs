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

pub struct FbxMeshData {
    pub vertex_data: VertexData,
    pub skin_data: Option<SkinData>,
    pub skeleton_id: Option<u32>,
    pub texture_path: Option<String>,
}

pub struct FbxLoadResult {
    pub meshes: Vec<FbxMeshData>,
    pub animation_system: AnimationSystem,
}

pub fn load_fbx_to_render_resources(path: &str) -> Result<FbxLoadResult> {
    let fbx_model = load_fbx_with_russimp(path)?;
    convert_fbx_model_to_render_resources(fbx_model)
}

fn convert_fbx_model_to_render_resources(fbx_model: FbxModel) -> Result<FbxLoadResult> {
    let mut animation_system = AnimationSystem::new();

    let skeleton_id = if !fbx_model.nodes.is_empty() {
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

    let mut meshes = Vec::new();
    for fbx_data in &fbx_model.fbx_data {
        let mesh_data = convert_fbx_data_to_mesh(fbx_data, skeleton_id, &animation_system);
        meshes.push(mesh_data);
    }

    if animation_system.clips.len() > 0 {
        animation_system.play(0);
    }

    Ok(FbxLoadResult {
        meshes,
        animation_system,
    })
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
) -> FbxMeshData {
    let mut vertices = Vec::with_capacity(fbx_data.positions.len());

    for i in 0..fbx_data.positions.len() {
        let pos = &fbx_data.positions[i];
        let normal = fbx_data.normals.get(i).cloned().unwrap_or(Vector3::new(0.0, 1.0, 0.0));
        let tex_coord = fbx_data.tex_coords.get(i).cloned().unwrap_or([0.5, 0.5]);

        vertices.push(Vertex {
            pos: Vec3::new(pos.x, pos.y, pos.z),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            tex_coord: Vec2::new(tex_coord[0], tex_coord[1]),
            normal: Vec3::new(normal.x, normal.y, normal.z),
        });
    }

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

    FbxMeshData {
        vertex_data,
        skin_data,
        skeleton_id,
        texture_path: fbx_data.diffuse_texture.clone(),
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
