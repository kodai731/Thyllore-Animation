use cgmath::{Matrix4, SquareMatrix};

use crate::{
    compose_transform, decompose_transform, BoneId, BoneLocalPose, Skeleton, SkeletonPose,
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
