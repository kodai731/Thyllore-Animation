use cgmath::{InnerSpace, Matrix3, Matrix4, Quaternion, Vector3};
use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::components::baked_motion::BakedMotion;
use thyllore_avatar_core::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};
use thyllore_math_core::quaternion_to_euler_degrees;

use crate::animation::{BoneId, Skeleton};
use crate::ecs::systems::avatar_setup_systems::{
    compute_bone_global_transform, skeleton_to_bone_inputs,
};

pub fn skeleton_to_retarget_skeleton(skeleton: &Skeleton) -> RetargetSkeleton {
    let bones = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(bone_index, bone)| {
            let world_matrix = compute_bone_global_transform(skeleton, bone_index);
            RetargetBone {
                parent: bone.parent_id.map(|id| id as usize),
                world_position: world_matrix.w.truncate(),
                world_rotation: extract_rotation(&world_matrix),
            }
        })
        .collect();
    RetargetSkeleton { bones }
}

pub fn recipe_to_clip(
    recipe_json: &str,
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
) -> anyhow::Result<EditableAnimationClip> {
    let recipe = thyllore_avatar_core::motion::systems::recipe_io::parse_recipe(recipe_json)?;

    let bones = skeleton_to_bone_inputs(skeleton);
    let frame = thyllore_avatar_core::humanoid::systems::character_frame::derive_character_frame(
        mapping, &bones,
    )
    .ok_or_else(|| anyhow::anyhow!("failed to derive character frame"))?;

    let rest_pose =
        thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose(mapping, &bones);

    let retarget_skeleton = skeleton_to_retarget_skeleton(skeleton);
    let ctx = thyllore_avatar_core::motion::systems::retarget_pose::build_retarget_context(
        &retarget_skeleton,
        mapping,
        &frame,
        rest_pose,
    )
    .ok_or_else(|| anyhow::anyhow!("failed to build retarget context"))?;

    let curves = thyllore_avatar_core::motion::systems::recipe_curves::build_recipe_curves(&recipe);
    let baked = thyllore_avatar_core::motion::systems::bake::bake_recipe_motion(&ctx, &curves);

    let hips_bone = *mapping
        .by_role
        .get(&HumanoidRole::Hips)
        .ok_or_else(|| anyhow::anyhow!("mapping has no Hips bone"))?;

    Ok(baked_motion_to_clip(
        &baked,
        skeleton,
        hips_bone,
        &recipe.name,
    ))
}

fn extract_rotation(matrix: &Matrix4<f32>) -> Quaternion<f32> {
    Quaternion::from(Matrix3::from_cols(
        matrix.x.truncate().normalize(),
        matrix.y.truncate().normalize(),
        matrix.z.truncate().normalize(),
    ))
}

fn unwrap_degrees(angle: f32, previous: f32) -> f32 {
    previous + (angle - previous + 180.0).rem_euclid(360.0) - 180.0
}

fn continuous_euler(euler: Vector3<f32>, prev: Option<Vector3<f32>>) -> Vector3<f32> {
    match prev {
        None => euler,
        Some(p) => Vector3::new(
            unwrap_degrees(euler.x, p.x),
            unwrap_degrees(euler.y, p.y),
            unwrap_degrees(euler.z, p.z),
        ),
    }
}

pub fn baked_motion_to_clip(
    baked: &BakedMotion,
    skeleton: &Skeleton,
    hips_bone: usize,
    clip_name: &str,
) -> EditableAnimationClip {
    let mut clip = EditableAnimationClip::new(0, clip_name.to_string());

    for (&bone_index, rotations) in &baked.bone_rotations {
        let bone_name = skeleton.bones[bone_index].name.clone();
        let track = clip.add_track(bone_index as BoneId, bone_name);

        let mut prev_euler: Option<Vector3<f32>> = None;
        for (&time, quat) in baked.frame_times.iter().zip(rotations) {
            let continuous = continuous_euler(quaternion_to_euler_degrees(quat), prev_euler);
            curve_add_keyframe(&mut track.rotation_x, time, continuous.x);
            curve_add_keyframe(&mut track.rotation_y, time, continuous.y);
            curve_add_keyframe(&mut track.rotation_z, time, continuous.z);
            prev_euler = Some(continuous);
        }

        if bone_index == hips_bone {
            let hips = &skeleton.bones[hips_bone];
            let bind_translation = hips.local_transform.w.truncate();
            let parent_world_rotation_inv = match hips.parent_id {
                Some(parent_id) => {
                    extract_rotation(&compute_bone_global_transform(skeleton, parent_id as usize))
                        .conjugate()
                }
                None => Quaternion::new(1.0, 0.0, 0.0, 0.0),
            };

            for (&time, offset) in baked.frame_times.iter().zip(&baked.hips_offsets) {
                let translation = bind_translation + parent_world_rotation_inv * *offset;
                curve_add_keyframe(&mut track.translation_x, time, translation.x);
                curve_add_keyframe(&mut track.translation_y, time, translation.y);
                curve_add_keyframe(&mut track.translation_z, time, translation.z);
            }
        }
    }

    if let Some(last_time) = baked.frame_times.last() {
        clip.duration = *last_time;
    }

    clip
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Rad, Rotation3};
    use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
    use thyllore_math_core::euler_degrees_to_quaternion;

    fn make_chain_skeleton(bone_count: usize) -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        for i in 0..bone_count {
            let parent_id = if i == 0 {
                None
            } else {
                Some((i - 1) as BoneId)
            };
            skeleton.add_bone(&format!("bone_{}", i), parent_id);
        }
        skeleton
    }

    #[test]
    fn test_quaternion_roundtrip() {
        let skeleton = make_chain_skeleton(3);
        let quaternions: Vec<Quaternion<f32>> = vec![
            Quaternion::new(1.0, 0.0, 0.0, 0.0),
            Quaternion::from_angle_x(Rad(90.0_f32.to_radians())),
            Quaternion::from_angle_y(Rad(45.0_f32.to_radians())),
        ];
        let fps = 30;
        let frame_times: Vec<f32> = (0..quaternions.len())
            .map(|i| i as f32 / fps as f32)
            .collect();
        let mut bone_rotations = std::collections::BTreeMap::new();
        bone_rotations.insert(0, quaternions.clone());
        let baked = BakedMotion {
            fps,
            frame_times,
            bone_rotations,
            hips_offsets: vec![],
            morph_weights: std::collections::BTreeMap::new(),
        };
        let clip = baked_motion_to_clip(&baked, &skeleton, 0, "test");
        let track = clip.get_track(0).expect("track for bone 0");

        for (i, quat) in quaternions.iter().enumerate() {
            let rx = track.rotation_x.keyframes[i].value;
            let ry = track.rotation_y.keyframes[i].value;
            let rz = track.rotation_z.keyframes[i].value;
            let reconstructed = euler_degrees_to_quaternion(&Vector3::new(rx, ry, rz));
            let dot = quat.dot(reconstructed);
            assert!(
                dot.abs() > 0.9999,
                "frame {}: dot={:.6}, quats not close",
                i,
                dot
            );
        }
    }

    #[test]
    fn test_continuous_euler_z_170_to_190() {
        let skeleton = make_chain_skeleton(1);
        let q1 = Quaternion::from_angle_z(Rad(170.0_f32.to_radians()));
        let q2 = Quaternion::from_angle_z(Rad(190.0_f32.to_radians()));
        let quaternions = vec![q1, q2];
        let fps = 30;
        let frame_times: Vec<f32> = (0..quaternions.len())
            .map(|i| i as f32 / fps as f32)
            .collect();
        let mut bone_rotations = std::collections::BTreeMap::new();
        bone_rotations.insert(0, quaternions);
        let baked = BakedMotion {
            fps,
            frame_times,
            bone_rotations,
            hips_offsets: vec![],
            morph_weights: std::collections::BTreeMap::new(),
        };
        let clip = baked_motion_to_clip(&baked, &skeleton, 0, "test");
        let track = clip.get_track(0).expect("track for bone 0");

        let z0 = track.rotation_z.keyframes[0].value;
        let z1 = track.rotation_z.keyframes[1].value;
        assert!(
            (z0 - 170.0).abs() < 0.01,
            "first key z={:.2}, expected ~170",
            z0
        );
        assert!(
            (z1 - 190.0).abs() < 0.01,
            "second key z={:.2}, expected ~190 (not -170)",
            z1
        );
    }

    #[test]
    fn test_hips_offset_translation() {
        let mut skeleton = Skeleton::new("test");
        skeleton.add_bone("Hips", None);
        let baked_offset = Vector3::new(1.0, 2.0, 3.0);
        let quaternions = vec![Quaternion::new(1.0, 0.0, 0.0, 0.0)];
        let fps = 30;
        let frame_times = vec![0.0];
        let mut bone_rotations = std::collections::BTreeMap::new();
        bone_rotations.insert(0, quaternions);
        let baked = BakedMotion {
            fps,
            frame_times,
            bone_rotations,
            hips_offsets: vec![baked_offset],
            morph_weights: std::collections::BTreeMap::new(),
        };
        let clip = baked_motion_to_clip(&baked, &skeleton, 0, "test");
        let track = clip.get_track(0).expect("track for Hips");

        let tx = track.translation_x.keyframes[0].value;
        let ty = track.translation_y.keyframes[0].value;
        let tz = track.translation_z.keyframes[0].value;
        assert!((tx - 1.0).abs() < 0.001, "tx={:.3}, expected 1.0", tx);
        assert!((ty - 2.0).abs() < 0.001, "ty={:.3}, expected 2.0", ty);
        assert!((tz - 3.0).abs() < 0.001, "tz={:.3}, expected 3.0", tz);
    }

    #[test]
    fn test_recipe_to_clip_wave() {
        use std::fs;
        use std::path::Path;

        let test_data_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/thyllore-avatar-core/tests/data");
        let fbx_txt_path = test_data_dir.join("rigs/mixamo.fbx.txt");
        let recipe_path = test_data_dir.join("recipes/wave.json");

        let fbx_path = std::env::temp_dir().join("motion_recipe_to_clip_mixamo.fbx");
        fs::copy(&fbx_txt_path, &fbx_path).unwrap();
        let load_result = thyllore_importer_core::fbx::loader::load_fbx_to_graphics_resources(
            fbx_path.to_str().unwrap(),
        );
        fs::remove_file(&fbx_path).ok();
        let (fbx_result, _) = load_result.expect("Failed to load FBX");
        let skeleton = fbx_result
            .animation_system
            .skeletons
            .first()
            .expect("No skeleton in loaded model")
            .clone();

        let bones = skeleton_to_bone_inputs(&skeleton);
        let (mapping, _) = infer_mapping(&bones);

        let recipe_json = fs::read_to_string(recipe_path).expect("Failed to read recipe");

        let clip = recipe_to_clip(&recipe_json, &skeleton, &mapping).unwrap();

        assert_eq!(clip.name, "wave_right_hand");
        assert!(
            (clip.duration - 2.8).abs() < 1e-4,
            "duration={:.4}, expected 2.8",
            clip.duration
        );

        let right_lower_arm_bone_idx = mapping
            .by_role
            .get(&HumanoidRole::RightLowerArm)
            .expect("RightLowerArm not found in mapping");
        let track = clip
            .get_track(*right_lower_arm_bone_idx as BoneId)
            .expect("RightLowerArm track not found");

        let key_count = track.rotation_x.keyframes.len();
        assert!(
            key_count == 85,
            "RightLowerArm rotation key count={}, expected 85 (2.8 * 30 + 1)",
            key_count
        );

        let hips_bone_idx = mapping
            .by_role
            .get(&HumanoidRole::Hips)
            .expect("Hips not found in mapping");
        let hips_track = clip
            .get_track(*hips_bone_idx as BoneId)
            .expect("Hips track not found");

        assert!(
            !hips_track.translation_x.keyframes.is_empty(),
            "Hips translation keys are empty"
        );
    }
}
