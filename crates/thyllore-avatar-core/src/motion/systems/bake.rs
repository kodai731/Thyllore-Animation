use std::collections::BTreeMap;

use cgmath::{InnerSpace, One, Quaternion};

use crate::motion::components::baked_motion::BakedMotion;
use crate::motion::components::recipe_curves::RecipeCurves;
use crate::motion::components::retarget_context::RetargetContext;

use super::recipe_curves::sample_recipe_pose;
use super::retarget_pose::retarget_pose;

pub fn bake_recipe_motion(ctx: &RetargetContext, curves: &RecipeCurves) -> BakedMotion {
    let fps = curves.fps as i32;
    let frame_count = (curves.duration_seconds * fps as f32).round() as usize + 1;

    let mut frame_times = Vec::with_capacity(frame_count);
    let mut bone_rotations: BTreeMap<usize, Vec<Quaternion<f32>>> = BTreeMap::new();
    let mut hips_offsets = Vec::with_capacity(frame_count);
    let mut morph_weights: BTreeMap<String, Vec<f32>> = BTreeMap::new();

    for k in 0..frame_count {
        let time = k as f32 / fps as f32;
        frame_times.push(time);

        let sampled = sample_recipe_pose(curves, time);
        let retargeted = retarget_pose(ctx, &sampled);

        for (&bone_idx, &rotation) in &retargeted.local_rotations {
            let curve = bone_rotations
                .entry(bone_idx)
                .or_insert_with(|| vec![Quaternion::one(); frame_count]);
            if k == 0 {
                curve[k] = rotation;
            } else {
                let prev = curve[k - 1];
                let dot = prev.dot(rotation);
                curve[k] = if dot < 0.0 { -rotation } else { rotation };
            }
        }

        hips_offsets.push(retargeted.hips_offset);

        for (name, &weight) in &sampled.morph {
            let curve = morph_weights
                .entry(name.clone())
                .or_insert_with(|| vec![0.0; frame_count]);
            curve[k] = weight;
        }
    }

    BakedMotion {
        fps: curves.fps,
        frame_times,
        bone_rotations,
        hips_offsets,
        morph_weights,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cgmath::{InnerSpace, One, Quaternion, Vector3};

    use crate::humanoid::components::character_frame::CharacterFrame;
    use crate::humanoid::components::mapping::HumanoidMapping;
    use crate::humanoid::components::rest_pose::RestPose;
    use crate::humanoid::components::role::HumanoidRole;
    use crate::motion::components::pose_recipe::{PoseRecipe, RecipePose};
    use crate::motion::components::recipe_curves::RecipeCurves;
    use crate::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};

    use super::*;
    use crate::motion::systems::recipe_curves::build_recipe_curves;
    use crate::motion::systems::retarget_pose::build_retarget_context;

    const HIPS: usize = 0;
    const SPINE: usize = 1;
    const HEAD: usize = 2;
    const LEFT_UPPER_ARM: usize = 3;
    const RIGHT_UPPER_ARM: usize = 4;
    const LEFT_LOWER_ARM: usize = 5;
    const RIGHT_LOWER_ARM: usize = 6;
    const LEFT_FOOT: usize = 7;
    const RIGHT_FOOT: usize = 8;

    fn default_frame() -> CharacterFrame {
        CharacterFrame {
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, -1.0],
        }
    }

    fn bone(parent: Option<usize>, x: f32, y: f32, z: f32) -> RetargetBone {
        RetargetBone {
            parent,
            world_position: Vector3::new(x, y, z),
            world_rotation: Quaternion::one(),
        }
    }

    fn build_skeleton() -> RetargetSkeleton {
        RetargetSkeleton {
            bones: vec![
                bone(None, 0.0, 1.0, 0.0),
                bone(Some(HIPS), 0.0, 1.3, 0.0),
                bone(Some(SPINE), 0.0, 1.7, 0.0),
                bone(Some(SPINE), -0.2, 1.5, 0.0),
                bone(Some(SPINE), 0.2, 1.5, 0.0),
                bone(Some(LEFT_UPPER_ARM), -0.5, 1.5, 0.0),
                bone(Some(RIGHT_UPPER_ARM), 0.5, 1.5, 0.0),
                bone(Some(HIPS), -0.1, 0.0, 0.0),
                bone(Some(HIPS), 0.1, 0.0, 0.0),
            ],
        }
    }

    fn build_mapping() -> HumanoidMapping {
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, HIPS);
        mapping.by_role.insert(HumanoidRole::Spine, SPINE);
        mapping.by_role.insert(HumanoidRole::Head, HEAD);
        mapping
            .by_role
            .insert(HumanoidRole::LeftUpperArm, LEFT_UPPER_ARM);
        mapping
            .by_role
            .insert(HumanoidRole::RightUpperArm, RIGHT_UPPER_ARM);
        mapping
            .by_role
            .insert(HumanoidRole::LeftLowerArm, LEFT_LOWER_ARM);
        mapping
            .by_role
            .insert(HumanoidRole::RightLowerArm, RIGHT_LOWER_ARM);
        mapping.by_role.insert(HumanoidRole::LeftFoot, LEFT_FOOT);
        mapping.by_role.insert(HumanoidRole::RightFoot, RIGHT_FOOT);
        mapping
    }

    fn build_ctx(skeleton: &RetargetSkeleton) -> RetargetContext {
        let mapping = build_mapping();
        let frame = default_frame();
        build_retarget_context(skeleton, &mapping, &frame, RestPose::TPose).unwrap()
    }

    fn make_recipe(duration: f32, fps: u32, poses: Vec<RecipePose>) -> RecipeCurves {
        let recipe = PoseRecipe {
            version: 1,
            name: "test".to_string(),
            prompt: String::new(),
            duration_seconds: duration,
            fps,
            is_loop: false,
            poses,
            cycle: None,
            hand_presets: BTreeMap::new(),
            copilot: Default::default(),
        };
        build_recipe_curves(&recipe)
    }

    #[test]
    fn test_frame_count_and_times() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton);

        let curves = make_recipe(
            1.0,
            30,
            vec![
                RecipePose {
                    time: 0.0,
                    ease: Default::default(),
                    rotations: BTreeMap::new(),
                    hips_translation: None,
                    morph: BTreeMap::new(),
                },
                RecipePose {
                    time: 1.0,
                    ease: Default::default(),
                    rotations: BTreeMap::new(),
                    hips_translation: None,
                    morph: BTreeMap::new(),
                },
            ],
        );

        let baked = bake_recipe_motion(&ctx, &curves);

        let expected_count = (1.0f32 * 30.0f32).round() as usize + 1;
        assert_eq!(baked.frame_times.len(), expected_count);
        assert_eq!(baked.hips_offsets.len(), expected_count);
        assert!((baked.frame_times[0] - 0.0).abs() < 1e-6);
        assert!((baked.frame_times[baked.frame_times.len() - 1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_quaternion_continuity() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton);

        let curves = make_recipe(
            1.0,
            30,
            vec![
                RecipePose {
                    time: 0.0,
                    ease: Default::default(),
                    rotations: BTreeMap::new(),
                    hips_translation: None,
                    morph: BTreeMap::new(),
                },
                RecipePose {
                    time: 1.0,
                    ease: Default::default(),
                    rotations: BTreeMap::new(),
                    hips_translation: None,
                    morph: BTreeMap::new(),
                },
            ],
        );

        let baked = bake_recipe_motion(&ctx, &curves);

        for (_bone_idx, curve) in &baked.bone_rotations {
            for i in 1..curve.len() {
                let dot = curve[i - 1].dot(curve[i]);
                assert!(
                    dot >= 0.0,
                    "quaternion dot product at frame {} is negative: {:.6}",
                    i,
                    dot
                );
            }
        }
    }

    #[test]
    fn test_sign_flip_continuity() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton);

        let mut rotations = BTreeMap::new();
        rotations.insert(HumanoidRole::Spine, [0.0, 0.0, 0.0]);
        let pose_start = RecipePose {
            time: 0.0,
            ease: Default::default(),
            rotations: rotations.clone(),
            hips_translation: None,
            morph: BTreeMap::new(),
        };

        let mut rotations_end = BTreeMap::new();
        rotations_end.insert(HumanoidRole::Spine, [0.0, 0.0, 350.0]);
        let pose_end = RecipePose {
            time: 1.0,
            ease: Default::default(),
            rotations: rotations_end,
            hips_translation: None,
            morph: BTreeMap::new(),
        };

        let curves = make_recipe(1.0, 30, vec![pose_start, pose_end]);

        let baked = bake_recipe_motion(&ctx, &curves);

        let spine_curve = baked.bone_rotations.get(&SPINE).unwrap();
        for i in 1..spine_curve.len() {
            let dot = spine_curve[i - 1].dot(spine_curve[i]);
            assert!(
                dot >= 0.0,
                "sign flip test: quaternion dot product at frame {} is negative: {:.6}",
                i,
                dot
            );
        }
    }
}
