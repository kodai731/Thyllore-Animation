use crate::scene::animation::{
    AnimationClip, AnimationSystem, Interpolation, Keyframe, Skeleton, SkinData, TransformChannel,
};
use crate::vulkanr::data::{Vertex, VertexData};
use crate::math::{Vec2, Vec3, Vec4};
use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Vector4};

use super::gltf::{GltfModel, GltfData, Joint};

pub struct GltfMeshData {
    pub vertex_data: VertexData,
    pub skin_data: Option<SkinData>,
    pub skeleton_id: Option<u32>,
    pub texture_path: Option<String>,
}

pub struct GltfLoadResult {
    pub meshes: Vec<GltfMeshData>,
    pub animation_system: AnimationSystem,
}

pub fn convert_gltf_to_render_resources(gltf_model: &GltfModel) -> GltfLoadResult {
    let mut animation_system = AnimationSystem::new();

    let skeleton_id = if !gltf_model.joints.is_empty() {
        let skeleton = convert_joints_to_skeleton(&gltf_model.joints, &gltf_model.skeleton_root_transform);
        Some(animation_system.add_skeleton(skeleton))
    } else {
        None
    };

    if !gltf_model.joint_animations.is_empty() {
        let clip = convert_joint_animations_to_clip(&gltf_model.joint_animations, &gltf_model.joints);
        crate::log!("Joint animation clip: duration={}, channels={}", clip.duration, clip.channels.len());
        if clip.duration > 0.0 && !clip.channels.is_empty() {
            animation_system.add_clip(clip);
        } else {
            crate::log!("Skipping joint animation clip (no valid data)");
        }
    }

    if !gltf_model.node_animations.is_empty() && skeleton_id.is_some() {
        let clip = convert_node_animations_to_clip(
            &gltf_model.node_animations,
            &gltf_model.rrnodes,
            &animation_system,
            skeleton_id.unwrap(),
        );
        crate::log!("Node animation clip: duration={}, channels={}", clip.duration, clip.channels.len());
        if clip.duration > 0.0 && !clip.channels.is_empty() {
            animation_system.add_clip(clip);
        } else {
            crate::log!("Skipping node animation clip (no valid data)");
        }
    }

    let mut meshes = Vec::new();
    let scale = if gltf_model.has_armature { 0.01 } else { 1.0 };
    crate::log!("glTF scale: {} (has_armature={}, has_skinned_meshes={})",
        scale, gltf_model.has_armature, gltf_model.has_skinned_meshes);

    for gltf_data in &gltf_model.gltf_data {
        let mut mesh_data = convert_gltf_data_to_mesh(
            gltf_data,
            skeleton_id,
            &gltf_model.joints,
            &animation_system,
        );

        if scale != 1.0 {
            for v in &mut mesh_data.vertex_data.vertices {
                v.pos.x *= scale;
                v.pos.y *= scale;
                v.pos.z *= scale;
            }
        }

        meshes.push(mesh_data);
    }

    log_gltf_scale_info(&meshes);

    if !animation_system.clips.is_empty() {
        animation_system.play(0);
    }

    GltfLoadResult {
        meshes,
        animation_system,
    }
}

fn log_gltf_scale_info(meshes: &[GltfMeshData]) {
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

        crate::log!("=== glTF Scale Info ===");
        crate::log!("  Total vertices: {}", total_vertices);
        crate::log!("  Bounding box min: ({:.4}, {:.4}, {:.4})", min_x, min_y, min_z);
        crate::log!("  Bounding box max: ({:.4}, {:.4}, {:.4})", max_x, max_y, max_z);
        crate::log!("  Size: ({:.4}, {:.4}, {:.4})", size_x, size_y, size_z);
        crate::log!("  Max dimension: {:.4} (glTF spec: meters)", max_dimension);

        if max_dimension > 100.0 {
            crate::log!("  WARNING: Model appears very large. Might be in mm or cm units.");
        } else if max_dimension < 0.01 {
            crate::log!("  WARNING: Model appears very small. Check unit scale.");
        }
    }
}

fn convert_joints_to_skeleton(joints: &[Joint], skeleton_root_transform: &Option<[[f32; 4]; 4]>) -> Skeleton {
    let mut skeleton = Skeleton::new("gltf_skeleton");

    if let Some(transform) = skeleton_root_transform {
        skeleton.root_transform = mat4_from_array(*transform);
        crate::log!("Skeleton root_transform set from glTF: diag=[{:.4}, {:.4}, {:.4}], trans=[{:.4}, {:.4}, {:.4}]",
            skeleton.root_transform[0][0], skeleton.root_transform[1][1], skeleton.root_transform[2][2],
            skeleton.root_transform[3][0], skeleton.root_transform[3][1], skeleton.root_transform[3][2]);
    } else {
        crate::log!("Skeleton root_transform: identity (no parent transform needed)");
    }

    for joint in joints {
        let parent_id = find_parent_joint_id(joints, joint.index);
        let bone_id = skeleton.add_bone(&joint.name, parent_id);

        if let Some(bone) = skeleton.get_bone_mut(bone_id) {
            bone.local_transform = mat4_from_array(joint.transform);
            bone.inverse_bind_pose = mat4_from_array(joint.inverse_bind_pose);
        }
    }

    skeleton
}

fn find_parent_joint_id(joints: &[Joint], child_index: u16) -> Option<u32> {
    for (idx, joint) in joints.iter().enumerate() {
        if joint.child_joint_indices.contains(&child_index) {
            return Some(idx as u32);
        }
    }
    None
}

fn convert_joint_animations_to_clip(
    joint_animations: &[Vec<super::gltf::JointAnimation>],
    _joints: &[Joint],
) -> AnimationClip {
    let mut clip = AnimationClip::new("gltf_joint_animation");
    let mut max_duration = 0.0f32;

    for (joint_idx, anims) in joint_animations.iter().enumerate() {
        if anims.is_empty() {
            continue;
        }

        let mut all_times: Vec<f32> = Vec::new();
        for anim in anims {
            for &time in &anim.key_frames {
                if !all_times.iter().any(|t| (*t - time).abs() < 0.0001) {
                    all_times.push(time);
                }
            }
        }
        all_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if let Some(&last) = all_times.last() {
            if last > max_duration {
                max_duration = last;
            }
        }

        let mut channel = TransformChannel::default();
        channel.interpolation = Interpolation::Step;

        for &time in &all_times {
            let mut combined_translate = Matrix4::identity();
            let mut combined_rotation = Matrix4::identity();
            let mut combined_scale = Matrix4::identity();

            for anim in anims {
                let key_frame_id = identify_key_frame_index_step(&anim.key_frames, time);

                if key_frame_id < anim.scales.len() {
                    combined_scale = mat4_from_array(array_from_mat4(&anim.scales[key_frame_id])) * combined_scale;
                }
                if key_frame_id < anim.rotations.len() {
                    combined_rotation = mat4_from_array(array_from_mat4(&anim.rotations[key_frame_id])) * combined_rotation;
                }
                if key_frame_id < anim.translations.len() {
                    combined_translate = mat4_from_array(array_from_mat4(&anim.translations[key_frame_id])) * combined_translate;
                }
            }

            channel.translation.push(Keyframe {
                time,
                value: Vector3::new(combined_translate[3][0], combined_translate[3][1], combined_translate[3][2]),
            });

            channel.rotation.push(Keyframe {
                time,
                value: matrix_to_quaternion(&combined_rotation),
            });

            channel.scale.push(Keyframe {
                time,
                value: Vector3::new(combined_scale[0][0], combined_scale[1][1], combined_scale[2][2]),
            });
        }

        if !channel.translation.is_empty() || !channel.rotation.is_empty() || !channel.scale.is_empty() {
            clip.add_channel(joint_idx as u32, channel);
        }
    }

    clip.duration = max_duration;
    clip
}

fn identify_key_frame_index_step(key_frames: &[f32], time: f32) -> usize {
    if key_frames.is_empty() {
        return 0;
    }
    let period = *key_frames.last().unwrap();
    if period <= 0.0 {
        return 0;
    }
    let time = time.rem_euclid(period);
    for (i, &key_frame) in key_frames.iter().enumerate() {
        if time < key_frame {
            return i;
        }
    }
    key_frames.len() - 1
}

fn array_from_mat4(m: &Matrix4<f32>) -> [[f32; 4]; 4] {
    [
        [m[0][0], m[0][1], m[0][2], m[0][3]],
        [m[1][0], m[1][1], m[1][2], m[1][3]],
        [m[2][0], m[2][1], m[2][2], m[2][3]],
        [m[3][0], m[3][1], m[3][2], m[3][3]],
    ]
}

fn convert_node_animations_to_clip(
    node_animations: &[super::gltf::NodeAnimation],
    rrnodes: &[super::gltf::RRNode],
    animation_system: &AnimationSystem,
    skeleton_id: u32,
) -> AnimationClip {
    let mut clip = AnimationClip::new("gltf_node_animation");

    let skeleton = match animation_system.get_skeleton(skeleton_id) {
        Some(s) => s,
        None => return clip,
    };

    let mut max_duration = 0.0f32;

    for node_anim in node_animations {
        let node = rrnodes.iter().find(|n| n.index as usize == node_anim.node_index);
        let bone_id = node.and_then(|n| skeleton.bone_name_to_id.get(&n.name).copied());

        let Some(bid) = bone_id else {
            continue;
        };

        let mut channel = TransformChannel::default();

        for (i, &time) in node_anim.translation_keyframes.iter().enumerate() {
            if i < node_anim.translations.len() {
                channel.translation.push(Keyframe {
                    time,
                    value: node_anim.translations[i],
                });
                if time > max_duration {
                    max_duration = time;
                }
            }
        }

        for (i, &time) in node_anim.rotation_keyframes.iter().enumerate() {
            if i < node_anim.rotations.len() {
                channel.rotation.push(Keyframe {
                    time,
                    value: node_anim.rotations[i],
                });
                if time > max_duration {
                    max_duration = time;
                }
            }
        }

        for (i, &time) in node_anim.scale_keyframes.iter().enumerate() {
            if i < node_anim.scales.len() {
                channel.scale.push(Keyframe {
                    time,
                    value: node_anim.scales[i],
                });
                if time > max_duration {
                    max_duration = time;
                }
            }
        }

        if !channel.translation.is_empty() || !channel.rotation.is_empty() || !channel.scale.is_empty() {
            clip.add_channel(bid, channel);
        }
    }

    clip.duration = max_duration;
    clip
}

fn convert_gltf_data_to_mesh(
    gltf_data: &GltfData,
    skeleton_id: Option<u32>,
    joints: &[Joint],
    animation_system: &AnimationSystem,
) -> GltfMeshData {
    let mut vertices = Vec::with_capacity(gltf_data.vertices.len());

    for v in &gltf_data.vertices {
        vertices.push(Vertex {
            pos: Vec3::new(v.position[0], v.position[1], v.position[2]),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            tex_coord: Vec2::new(v.tex_coord[0], v.tex_coord[1]),
            normal: Vec3::new(v.normal[0], v.normal[1], v.normal[2]),
        });
    }

    let vertex_data = VertexData {
        vertices,
        indices: gltf_data.indices.clone(),
    };

    let skin_data = if !joints.is_empty() && skeleton_id.is_some() && gltf_data.has_joints {
        let skeleton = animation_system.get_skeleton(skeleton_id.unwrap());
        Some(convert_gltf_to_skin_data(gltf_data, skeleton_id.unwrap(), skeleton))
    } else {
        None
    };

    GltfMeshData {
        vertex_data,
        skin_data,
        skeleton_id,
        texture_path: None,
    }
}

fn convert_gltf_to_skin_data(
    gltf_data: &GltfData,
    skeleton_id: u32,
    _skeleton: Option<&Skeleton>,
) -> SkinData {
    let vertex_count = gltf_data.vertices.len();

    let mut bone_indices: Vec<Vector4<u32>> = Vec::with_capacity(vertex_count);
    let mut bone_weights: Vec<Vector4<f32>> = Vec::with_capacity(vertex_count);
    let mut base_positions: Vec<Vector3<f32>> = Vec::with_capacity(vertex_count);
    let mut base_normals: Vec<Vector3<f32>> = Vec::with_capacity(vertex_count);

    for v in &gltf_data.vertices {
        bone_indices.push(Vector4::new(
            v.joint_indices[0] as u32,
            v.joint_indices[1] as u32,
            v.joint_indices[2] as u32,
            v.joint_indices[3] as u32,
        ));
        bone_weights.push(Vector4::new(
            v.joint_weights[0],
            v.joint_weights[1],
            v.joint_weights[2],
            v.joint_weights[3],
        ));
        base_positions.push(Vector3::new(
            v.animation_position[0],
            v.animation_position[1],
            v.animation_position[2],
        ));
        base_normals.push(Vector3::new(
            v.normal[0],
            v.normal[1],
            v.normal[2],
        ));
    }

    SkinData {
        skeleton_id,
        bone_indices,
        bone_weights,
        base_positions,
        base_normals,
    }
}

fn mat4_from_array(arr: [[f32; 4]; 4]) -> Matrix4<f32> {
    Matrix4::from_cols(
        Vector4::new(arr[0][0], arr[0][1], arr[0][2], arr[0][3]),
        Vector4::new(arr[1][0], arr[1][1], arr[1][2], arr[1][3]),
        Vector4::new(arr[2][0], arr[2][1], arr[2][2], arr[2][3]),
        Vector4::new(arr[3][0], arr[3][1], arr[3][2], arr[3][3]),
    )
}

fn matrix_to_quaternion(m: &Matrix4<f32>) -> Quaternion<f32> {
    let trace = m[0][0] + m[1][1] + m[2][2];

    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        Quaternion::new(
            0.25 * s,
            (m[1][2] - m[2][1]) / s,
            (m[2][0] - m[0][2]) / s,
            (m[0][1] - m[1][0]) / s,
        )
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        Quaternion::new(
            (m[1][2] - m[2][1]) / s,
            0.25 * s,
            (m[1][0] + m[0][1]) / s,
            (m[2][0] + m[0][2]) / s,
        )
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        Quaternion::new(
            (m[2][0] - m[0][2]) / s,
            (m[1][0] + m[0][1]) / s,
            0.25 * s,
            (m[2][1] + m[1][2]) / s,
        )
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        Quaternion::new(
            (m[0][1] - m[1][0]) / s,
            (m[2][0] + m[0][2]) / s,
            (m[2][1] + m[1][2]) / s,
            0.25 * s,
        )
    }
}
