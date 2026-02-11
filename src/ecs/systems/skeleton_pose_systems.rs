use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};

use crate::animation::{
    compose_transform, decompose_transform, AnimationClip, BoneId, BoneLocalPose, Skeleton,
    SkeletonPose, SkinData,
};

pub fn create_pose_from_rest(skeleton: &Skeleton) -> SkeletonPose {
    let bone_poses = skeleton
        .bones
        .iter()
        .map(|bone| {
            let (t, r, s) = decompose_transform(&bone.local_transform);
            BoneLocalPose {
                translation: t,
                rotation: r,
                scale: s,
            }
        })
        .collect();

    SkeletonPose {
        skeleton_id: skeleton.id,
        bone_poses,
    }
}

pub fn sample_clip_to_pose(
    clip: &AnimationClip,
    time: f32,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    looping: bool,
) {
    for (&bone_id, channel) in &clip.channels {
        let idx = bone_id as usize;
        if idx >= pose.bone_poses.len() {
            continue;
        }

        let rest = &skeleton.bones.get(idx);
        let (rest_t, rest_r, rest_s) = match rest {
            Some(bone) => decompose_transform(&bone.local_transform),
            None => continue,
        };

        let (translation, rotation, scale) = if looping && clip.duration > 0.0 {
            (
                channel
                    .sample_translation_looped(time, clip.duration)
                    .unwrap_or(rest_t),
                channel
                    .sample_rotation_looped(time, clip.duration)
                    .unwrap_or(rest_r),
                channel
                    .sample_scale_looped(time, clip.duration)
                    .unwrap_or(rest_s),
            )
        } else {
            (
                channel.sample_translation(time).unwrap_or(rest_t),
                channel.sample_rotation(time).unwrap_or(rest_r),
                channel.sample_scale(time).unwrap_or(rest_s),
            )
        };

        pose.bone_poses[idx] = BoneLocalPose {
            translation,
            rotation,
            scale,
        };
    }
}

pub fn compute_pose_global_transforms(
    skeleton: &Skeleton,
    pose: &SkeletonPose,
) -> Vec<Matrix4<f32>> {
    let bone_count = skeleton.bones.len();
    let mut global_transforms = vec![Matrix4::identity(); bone_count];

    fn compute_recursive(
        skeleton: &Skeleton,
        pose: &SkeletonPose,
        bone_id: BoneId,
        parent_transform: Matrix4<f32>,
        global_transforms: &mut [Matrix4<f32>],
    ) {
        let idx = bone_id as usize;
        let Some(bone) = skeleton.get_bone(bone_id) else {
            return;
        };

        let local = if idx < pose.bone_poses.len() {
            let bp = &pose.bone_poses[idx];
            compose_transform(bp.translation, bp.rotation, bp.scale)
        } else {
            bone.local_transform
        };

        let global = parent_transform * local;
        global_transforms[idx] = global;

        for &child_id in &bone.children {
            compute_recursive(skeleton, pose, child_id, global, global_transforms);
        }
    }

    for &root_id in &skeleton.root_bone_ids {
        compute_recursive(
            skeleton,
            pose,
            root_id,
            skeleton.root_transform,
            &mut global_transforms,
        );
    }

    global_transforms
}

pub fn compute_rest_global_transforms(skeleton: &Skeleton) -> Vec<Matrix4<f32>> {
    let bone_count = skeleton.bones.len();
    let mut global_transforms = vec![Matrix4::identity(); bone_count];

    fn compute_recursive(
        skeleton: &Skeleton,
        bone_id: BoneId,
        parent_transform: Matrix4<f32>,
        global_transforms: &mut [Matrix4<f32>],
    ) {
        let Some(bone) = skeleton.get_bone(bone_id) else {
            return;
        };

        let global = parent_transform * bone.local_transform;
        global_transforms[bone_id as usize] = global;

        for &child_id in &bone.children {
            compute_recursive(skeleton, child_id, global, global_transforms);
        }
    }

    for &root_id in &skeleton.root_bone_ids {
        compute_recursive(
            skeleton,
            root_id,
            skeleton.root_transform,
            &mut global_transforms,
        );
    }

    global_transforms
}

pub fn apply_skinning(
    skin_data: &SkinData,
    global_transforms: &[Matrix4<f32>],
    skeleton: &Skeleton,
    out_positions: &mut [Vector3<f32>],
    out_normals: &mut [Vector3<f32>],
) {
    let mut skin_matrices = Vec::with_capacity(skeleton.bone_count());

    for bone in &skeleton.bones {
        let global = global_transforms[bone.id as usize];
        let skin_matrix = global * bone.inverse_bind_pose;
        skin_matrices.push(skin_matrix);
    }

    for i in 0..skin_data.base_positions.len() {
        let indices = &skin_data.bone_indices[i];
        let weights = &skin_data.bone_weights[i];

        let mut skinned_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut skinned_normal = Vector3::new(0.0, 0.0, 0.0);

        for j in 0..4 {
            let bone_idx = match j {
                0 => indices.x,
                1 => indices.y,
                2 => indices.z,
                3 => indices.w,
                _ => 0,
            } as usize;

            let weight = match j {
                0 => weights.x,
                1 => weights.y,
                2 => weights.z,
                3 => weights.w,
                _ => 0.0,
            };

            if weight > 0.0 && bone_idx < skin_matrices.len() {
                let m = &skin_matrices[bone_idx];

                let pos = skin_data.base_positions[i];
                let transformed = m * Vector4::new(pos.x, pos.y, pos.z, 1.0);
                skinned_pos += Vector3::new(transformed.x, transformed.y, transformed.z) * weight;

                if i < skin_data.base_normals.len() {
                    let normal = skin_data.base_normals[i];
                    let transformed_n = m * Vector4::new(normal.x, normal.y, normal.z, 0.0);
                    skinned_normal +=
                        Vector3::new(transformed_n.x, transformed_n.y, transformed_n.z) * weight;
                }
            }
        }

        if i < out_positions.len() {
            out_positions[i] = skinned_pos;
        }

        if i < out_normals.len() {
            let len = (skinned_normal.x * skinned_normal.x
                + skinned_normal.y * skinned_normal.y
                + skinned_normal.z * skinned_normal.z)
                .sqrt();
            if len > 0.0 {
                out_normals[i] = skinned_normal / len;
            }
        }
    }
}
