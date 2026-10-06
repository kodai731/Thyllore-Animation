#![cfg(test)]
use std::collections::BTreeMap;

use cgmath::{InnerSpace, Vector3};

mod support;

fn check_with_world(convention_id: &str) {
    let (bones, skeleton) = support::fixture_bones::load_fixture_rig(convention_id);

    let (mapping, _) = thyllore_avatar_core::humanoid::systems::name_match::infer_mapping(&bones);

    let frame = thyllore_avatar_core::humanoid::systems::character_frame::derive_character_frame(
        &mapping, &bones,
    )
    .unwrap();

    let rest_pose =
        thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose(&mapping, &bones);

    let ctx = thyllore_avatar_core::motion::systems::retarget_pose::build_retarget_context(
        &skeleton, &mapping, &frame, rest_pose,
    )
    .unwrap();

    let pose = thyllore_avatar_core::motion::components::sampled_pose::SampledPose {
        rotations: BTreeMap::new(),
        hips_translation: [0.0, 0.0, 0.0],
        morph: BTreeMap::new(),
    };

    let retargeted =
        thyllore_avatar_core::motion::systems::retarget_pose::retarget_to_bones(&ctx, &pose);
    let positions = thyllore_avatar_core::motion::systems::retarget_pose::pose_world_positions(
        &ctx,
        &retargeted,
    );

    let hips_idx =
        mapping.by_role[&thyllore_avatar_core::humanoid::components::role::HumanoidRole::Hips];
    let hips_bind = skeleton.bones[hips_idx].world_position;
    let hips_height = ctx.hips_height;

    let canonical = support::canonical_bones();

    for (_i, cb) in canonical.iter().enumerate() {
        let role: thyllore_avatar_core::humanoid::components::role::HumanoidRole =
            humanoid_role_from_role_name(cb.role);

        let Some(&bone_idx) = mapping.by_role.get(&role) else {
            continue;
        };
        let p = positions[bone_idx];
        let char_pos =
            frame.to_character([p.x - hips_bind.x, p.y - hips_bind.y, p.z - hips_bind.z]);
        let normalized = [
            char_pos[0] / hips_height,
            char_pos[1] / hips_height,
            char_pos[2] / hips_height,
        ];

        let canonical_hips = canonical[0].position;
        let canonical_foot_y = canonical
            .iter()
            .enumerate()
            .filter(|(_, b)| b.role == "LeftFoot" || b.role == "RightFoot")
            .map(|(_, b)| b.position[1])
            .fold(f32::INFINITY, f32::min);
        let canonical_hips_height = canonical_hips[1] - canonical_foot_y;
        let expected_diff = [
            (cb.position[0] - canonical_hips[0]) / canonical_hips_height,
            (cb.position[1] - canonical_hips[1]) / canonical_hips_height,
            (cb.position[2] - canonical_hips[2]) / canonical_hips_height,
        ];

        let diff = Vector3::new(
            normalized[0] - expected_diff[0],
            normalized[1] - expected_diff[1],
            normalized[2] - expected_diff[2],
        );
        let err = diff.magnitude();
        assert!(
            err < 1e-3,
            "{}: role {:?} error {:.6} (got [{:.4},{:.4},{:.4}], expected [{:.4},{:.4},{:.4}])",
            convention_id,
            cb.role,
            err,
            normalized[0],
            normalized[1],
            normalized[2],
            expected_diff[0],
            expected_diff[1],
            expected_diff[2],
        );
    }
}

fn humanoid_role_from_role_name(
    name: &str,
) -> thyllore_avatar_core::humanoid::components::role::HumanoidRole {
    let role = thyllore_avatar_core::humanoid::components::role::HumanoidRole::ALL
        .iter()
        .find(|r| format!("{:?}", r) == name)
        .unwrap();
    *role
}

#[test]
fn test_all_conventions() {
    for convention_id in support::rig_names::CONVENTIONS.iter().copied() {
        check_with_world(convention_id);
    }
}
