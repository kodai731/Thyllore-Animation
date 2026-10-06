#![cfg(test)]
use cgmath::{InnerSpace, Vector3};
use thyllore_anim_core::editable::components::clip::EditableAnimationClip;

mod support;

fn check_clip_against_all_conventions(clip: &EditableAnimationClip) {
    for convention_id in support::rig_names::CONVENTIONS.iter().copied() {
        check_one(convention_id, clip);
    }
}

fn check_one(convention_id: &str, clip: &EditableAnimationClip) {
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

    let hips_idx =
        mapping.by_role[&thyllore_avatar_core::humanoid::components::role::HumanoidRole::Hips];
    let hips_bind = skeleton.bones[hips_idx].world_position;
    let hips_height = ctx.hips_height;

    let total_frames = (clip.duration * 30.0) as usize;
    for frame_idx in 0..=total_frames {
        let time = frame_idx as f32 / 30.0;
        let sampled =
            thyllore_avatar_core::motion::systems::role_clip_sampler::sample_role_clip(clip, time);

        let retargeted =
            thyllore_avatar_core::motion::systems::retarget_pose::retarget_to_bones(&ctx, &sampled);
        let positions = thyllore_avatar_core::motion::systems::retarget_pose::pose_world_positions(
            &ctx,
            &retargeted,
        );

        let reference = support::reference_pose::reference_world_positions(&sampled);

        for (role, ref_pos) in &reference {
            let Some(&bone_idx) = mapping.by_role.get(role) else {
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

            let canonical_hips = support::reference_pose::canonical_hips();
            let canonical_hips_height = support::reference_pose::canonical_hips_height();
            let expected_diff = [
                (ref_pos[0] - canonical_hips[0]) / canonical_hips_height,
                (ref_pos[1] - canonical_hips[1]) / canonical_hips_height,
                (ref_pos[2] - canonical_hips[2]) / canonical_hips_height,
            ];

            let diff = Vector3::new(
                normalized[0] - expected_diff[0],
                normalized[1] - expected_diff[1],
                normalized[2] - expected_diff[2],
            );
            let err = diff.magnitude();
            assert!(
                err < 1e-3,
                "convention {} frame {} time {:.2} role {:?} error {:.6} (got [{:.4},{:.4},{:.4}], expected [{:.4},{:.4},{:.4}])",
                convention_id,
                frame_idx,
                time,
                role,
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
}

#[test]
fn test_wave_invariance() {
    check_clip_against_all_conventions(&support::role_clips::wave());
}

#[test]
fn test_mixed_invariance() {
    check_clip_against_all_conventions(&support::role_clips::mixed());
}

#[test]
fn test_hands_on_hips_tilt_invariance() {
    check_clip_against_all_conventions(&support::role_clips::hands_on_hips_tilt());
}

#[test]
fn test_bow_invariance() {
    check_clip_against_all_conventions(&support::role_clips::bow());
}
