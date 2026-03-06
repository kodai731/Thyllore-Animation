use std::collections::HashMap;

use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3};

use crate::animation::{
    compose_transform, decompose_transform, BoneId, BoneLocalPose, Skeleton, SkeletonPose,
};

pub fn compute_local_override_from_global_translation(
    skeleton: &Skeleton,
    cached_globals: &[Matrix4<f32>],
    bone_id: BoneId,
    new_global_pos: Vector3<f32>,
) -> Option<BoneLocalPose> {
    let idx = bone_id as usize;
    if idx >= cached_globals.len() {
        return None;
    }

    let current_global = cached_globals[idx];
    let (_, current_rot, current_scale) = decompose_transform(&current_global);

    let new_global = compose_transform(new_global_pos, current_rot, current_scale);
    compute_local_from_global(skeleton, cached_globals, bone_id, new_global)
}

pub fn compute_local_override_from_global_rotation(
    skeleton: &Skeleton,
    cached_globals: &[Matrix4<f32>],
    bone_id: BoneId,
    gizmo_pos: Vector3<f32>,
    rotation: Quaternion<f32>,
) -> Option<BoneLocalPose> {
    let idx = bone_id as usize;
    if idx >= cached_globals.len() {
        return None;
    }

    let current_global = cached_globals[idx];
    let rot_mat: Matrix4<f32> = rotation.into();
    let translate_to_origin = Matrix4::from_translation(-gizmo_pos);
    let translate_back = Matrix4::from_translation(gizmo_pos);
    let new_global = translate_back * rot_mat * translate_to_origin * current_global;

    compute_local_from_global(skeleton, cached_globals, bone_id, new_global)
}

pub fn compute_local_override_from_global_scale(
    skeleton: &Skeleton,
    cached_globals: &[Matrix4<f32>],
    bone_id: BoneId,
    scale: Vector3<f32>,
) -> Option<BoneLocalPose> {
    let idx = bone_id as usize;
    if idx >= cached_globals.len() {
        return None;
    }

    let current_global = cached_globals[idx];
    let scale_mat = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
    let new_global = current_global * scale_mat;

    compute_local_from_global(skeleton, cached_globals, bone_id, new_global)
}

fn compute_local_from_global(
    skeleton: &Skeleton,
    cached_globals: &[Matrix4<f32>],
    bone_id: BoneId,
    new_global: Matrix4<f32>,
) -> Option<BoneLocalPose> {
    let bone = skeleton.get_bone(bone_id)?;

    let parent_global = match bone.parent_id {
        Some(parent_id) => {
            let parent_idx = parent_id as usize;
            if parent_idx < cached_globals.len() {
                cached_globals[parent_idx]
            } else {
                skeleton.root_transform
            }
        }
        None => skeleton.root_transform,
    };

    let parent_inverse = parent_global.invert().unwrap_or(Matrix4::identity());
    let local_matrix = parent_inverse * new_global;
    let (translation, rotation, scale) = decompose_transform(&local_matrix);

    Some(BoneLocalPose {
        translation,
        rotation,
        scale,
    })
}

pub fn apply_pose_overrides(pose: &mut SkeletonPose, overrides: &HashMap<BoneId, BoneLocalPose>) {
    for (&bone_id, local_pose) in overrides {
        let idx = bone_id as usize;
        if idx < pose.bone_poses.len() {
            pose.bone_poses[idx] = local_pose.clone();
        }
    }
}
