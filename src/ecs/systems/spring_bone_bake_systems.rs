use crate::animation::editable::{quaternion_to_euler_degrees, EditableAnimationClip};
use crate::animation::{AnimationClip, BoneId, Skeleton};
use crate::ecs::component::{ConstraintSet, SpringBoneSetup};

use super::constraint_solve_systems::apply_constraints;
use super::skeleton_pose_systems::{
    compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose,
};
use super::spring_bone_systems::{
    collect_affected_bone_ids, spring_bone_initialize, spring_bone_update,
    spring_bone_write_back_to_pose,
};

pub struct BakeConfig {
    pub start_time: f32,
    pub end_time: f32,
    pub sample_rate: f32,
}

impl Default for BakeConfig {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            end_time: 1.0,
            sample_rate: 30.0,
        }
    }
}

pub struct BakeResult {
    pub clip: EditableAnimationClip,
    pub baked_bone_ids: Vec<BoneId>,
}

pub fn spring_bone_bake(
    config: &BakeConfig,
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    base_clip: &AnimationClip,
    constraints: Option<&ConstraintSet>,
    looping: bool,
) -> BakeResult {
    let affected_ids = collect_affected_bone_ids(setup);
    if affected_ids.is_empty() {
        return BakeResult {
            clip: EditableAnimationClip::new(0, "spring_baked".to_string()),
            baked_bone_ids: Vec::new(),
        };
    }

    let mut pose = create_pose_from_rest(skeleton);
    let globals = compute_pose_global_transforms(skeleton, &pose);
    let mut sb_state = spring_bone_initialize(setup, skeleton, &globals);

    let mut editable = EditableAnimationClip::new(0, format!("{}_spring_baked", base_clip.name));

    for &bone_id in &affected_ids {
        let bone_name = skeleton
            .get_bone(bone_id)
            .map(|b| b.name.clone())
            .unwrap_or_else(|| format!("Bone_{}", bone_id));
        editable.add_track(bone_id, bone_name);
    }

    let frame_count = ((config.end_time - config.start_time) * config.sample_rate) as usize + 1;
    let dt = 1.0 / config.sample_rate;

    for frame_index in 0..frame_count {
        let sample_time = config.start_time + frame_index as f32 * dt;

        pose = create_pose_from_rest(skeleton);
        sample_clip_to_pose(base_clip, sample_time, skeleton, &mut pose, looping);

        if let Some(cs) = constraints {
            apply_constraints(cs, skeleton, &mut pose);
        }

        let mut globals = compute_pose_global_transforms(skeleton, &pose);

        spring_bone_update(setup, &mut sb_state, skeleton, &mut globals, &pose, dt);
        spring_bone_write_back_to_pose(skeleton, &globals, &mut pose, &affected_ids);

        capture_rotation_keyframes(&pose, sample_time, &affected_ids, &mut editable);
    }

    editable.duration = config.end_time - config.start_time;

    BakeResult {
        clip: editable,
        baked_bone_ids: affected_ids,
    }
}

pub fn merge_bake_into_clip(
    target: &mut EditableAnimationClip,
    bake_result: &BakeResult,
    skeleton: &Skeleton,
) {
    for &bone_id in &bake_result.baked_bone_ids {
        let Some(source_track) = bake_result.clip.get_track(bone_id) else {
            continue;
        };

        let track = if target.tracks.contains_key(&bone_id) {
            let t = target.tracks.get_mut(&bone_id).unwrap();
            t.rotation_x.keyframes.clear();
            t.rotation_y.keyframes.clear();
            t.rotation_z.keyframes.clear();
            t
        } else {
            let bone_name = skeleton
                .get_bone(bone_id)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| format!("Bone_{}", bone_id));
            target.add_track(bone_id, bone_name)
        };

        for kf in &source_track.rotation_x.keyframes {
            track.rotation_x.add_keyframe(kf.time, kf.value);
        }
        for kf in &source_track.rotation_y.keyframes {
            track.rotation_y.add_keyframe(kf.time, kf.value);
        }
        for kf in &source_track.rotation_z.keyframes {
            track.rotation_z.add_keyframe(kf.time, kf.value);
        }
    }
}

pub fn clear_baked_tracks(clip: &mut EditableAnimationClip, baked_bone_ids: &[BoneId]) {
    for &bone_id in baked_bone_ids {
        if let Some(track) = clip.tracks.get_mut(&bone_id) {
            track.rotation_x.keyframes.clear();
            track.rotation_y.keyframes.clear();
            track.rotation_z.keyframes.clear();
        }
    }
}

fn capture_rotation_keyframes(
    pose: &crate::animation::SkeletonPose,
    time: f32,
    affected_ids: &[BoneId],
    clip: &mut EditableAnimationClip,
) {
    for &bone_id in affected_ids {
        let idx = bone_id as usize;
        if idx >= pose.bone_poses.len() {
            continue;
        }

        let euler = quaternion_to_euler_degrees(&pose.bone_poses[idx].rotation);

        if let Some(track) = clip.tracks.get_mut(&bone_id) {
            track.rotation_x.add_keyframe(time, euler.x);
            track.rotation_y.add_keyframe(time, euler.y);
            track.rotation_z.add_keyframe(time, euler.z);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationClip, Bone, Skeleton};
    use crate::ecs::component::{SpringBoneSetup, SpringChain, SpringJointParam};
    use cgmath::{Matrix4, Vector3};

    fn create_test_skeleton() -> Skeleton {
        let bones = vec![
            Bone {
                id: 0,
                name: "Root".to_string(),
                parent_id: None,
                children: vec![1],
                local_transform: Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0)),
                ..Default::default()
            },
            Bone {
                id: 1,
                name: "Spring".to_string(),
                parent_id: Some(0),
                children: vec![2],
                local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
                ..Default::default()
            },
            Bone {
                id: 2,
                name: "SpringTip".to_string(),
                parent_id: Some(1),
                children: vec![],
                local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
                ..Default::default()
            },
        ];

        let mut bone_name_to_id = std::collections::HashMap::new();
        for bone in &bones {
            bone_name_to_id.insert(bone.name.clone(), bone.id);
        }

        Skeleton {
            id: 0,
            name: "TestSkeleton".to_string(),
            bones,
            bone_name_to_id,
            root_bone_ids: vec![0],
            root_transform: Matrix4::from_scale(1.0),
        }
    }

    fn create_test_setup() -> SpringBoneSetup {
        SpringBoneSetup {
            chains: vec![SpringChain {
                id: 0,
                name: "TestChain".to_string(),
                joints: vec![SpringJointParam {
                    bone_id: 1,
                    stiffness: 1.0,
                    gravity_power: 0.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    drag_force: 1.0,
                    hit_radius: 0.0,
                }],
                collider_group_ids: vec![],
                center_bone_id: None,
                enabled: true,
            }],
            colliders: vec![],
            collider_groups: vec![],
            next_chain_id: 1,
            next_collider_id: 0,
            next_group_id: 0,
        }
    }

    #[test]
    fn test_bake_frame_count() {
        let skeleton = create_test_skeleton();
        let setup = create_test_setup();
        let clip = AnimationClip::new("test");

        let config = BakeConfig {
            start_time: 0.0,
            end_time: 1.0,
            sample_rate: 30.0,
        };

        let result = spring_bone_bake(&config, &setup, &skeleton, &clip, None, false);

        let track = result.clip.get_track(1).unwrap();
        assert_eq!(track.rotation_x.keyframes.len(), 31);
        assert_eq!(track.rotation_y.keyframes.len(), 31);
        assert_eq!(track.rotation_z.keyframes.len(), 31);
    }

    #[test]
    fn test_bake_empty_chains() {
        let skeleton = create_test_skeleton();
        let setup = SpringBoneSetup {
            chains: vec![],
            colliders: vec![],
            collider_groups: vec![],
            next_chain_id: 0,
            next_collider_id: 0,
            next_group_id: 0,
        };
        let clip = AnimationClip::new("test");

        let config = BakeConfig {
            start_time: 0.0,
            end_time: 1.0,
            sample_rate: 30.0,
        };

        let result = spring_bone_bake(&config, &setup, &skeleton, &clip, None, false);

        assert!(result.baked_bone_ids.is_empty());
        assert!(result.clip.tracks.is_empty());
    }

    #[test]
    fn test_bake_stiff_spring_preserves_rotation() {
        let skeleton = create_test_skeleton();
        let setup = create_test_setup();
        let clip = AnimationClip::new("test");

        let config = BakeConfig {
            start_time: 0.0,
            end_time: 0.1,
            sample_rate: 30.0,
        };

        let result = spring_bone_bake(&config, &setup, &skeleton, &clip, None, false);

        let track = result.clip.get_track(1).unwrap();
        for kf in &track.rotation_x.keyframes {
            assert!(
                kf.value.abs() < 5.0,
                "rotation_x deviated too much: {}",
                kf.value
            );
        }
        for kf in &track.rotation_z.keyframes {
            assert!(
                kf.value.abs() < 5.0,
                "rotation_z deviated too much: {}",
                kf.value
            );
        }
    }
}
