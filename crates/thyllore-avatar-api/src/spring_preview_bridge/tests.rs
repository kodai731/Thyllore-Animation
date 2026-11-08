use glam::{Quat, Vec3};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring;

use super::*;
use crate::avatar_setup_bridge::BoneSpec;
use crate::spring_edit_bridge::{use_existing_bones_as_springs, UseExistingBonesAsSpringsRequest};

fn bone(name: &str, parent: Option<usize>, rest_position: [f32; 3]) -> BoneSpec {
    BoneSpec {
        name: name.to_string(),
        parent,
        rest_position,
        rest_frame: None,
    }
}

fn to_bone_input(specs: &[BoneSpec]) -> Vec<BoneInput> {
    specs
        .iter()
        .map(|s| BoneInput {
            name: s.name.clone(),
            parent: s.parent,
            rest_position: s.rest_position,
        })
        .collect()
}

/// Head at the origin with a three-bone strand hanging straight down the -Y axis.
fn hanging_strand() -> (Vec<BoneInput>, Vec<MatrixRows>, SidecarSpring) {
    let bone_specs = vec![
        bone("Head", None, [0.0, 0.0, 0.0]),
        bone("hair_a", Some(0), [0.0, -0.1, 0.0]),
        bone("hair_a.001", Some(1), [0.0, -0.2, 0.0]),
        bone("hair_a.002", Some(2), [0.0, -0.3, 0.0]),
    ];
    let rest_locals = vec![
        translation_rows(Vec3::new(0.0, 0.0, 0.0)),
        translation_rows(Vec3::new(0.0, -0.1, 0.0)),
        translation_rows(Vec3::new(0.0, -0.1, 0.0)),
        translation_rows(Vec3::new(0.0, -0.1, 0.0)),
    ];
    let request = UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: bone_specs,
        root_bone: "Head".to_string(),
        min_joints: 2,
    };
    let spring = use_existing_bones_as_springs(&request).unwrap();
    (to_bone_input(&request.bones), rest_locals, spring)
}

fn step_frames(
    preview: &mut SpringPreview,
    locals: &[MatrixRows],
    frames: usize,
) -> Vec<JointPose> {
    let mut last = Vec::new();
    for _ in 0..frames {
        last = preview.step(locals, 1.0 / 30.0).unwrap();
    }
    last
}

fn angle_degrees(rotation: [f32; 4]) -> f32 {
    2.0 * rotation[0].clamp(-1.0, 1.0).acos().to_degrees()
}

#[test]
fn spring_preview_strand_at_rest_under_gravity_stays_still() {
    let (bones, rest_locals, spring) = hanging_strand();
    let mut preview = SpringPreview::new(&spring, &bones, &rest_locals).unwrap();

    let rotations = step_frames(&mut preview, &rest_locals, 30);

    assert_eq!(rotations.len(), 3);
    for rotation in &rotations {
        assert!(angle_degrees(rotation.rotation) < 1e-2, "{rotation:?}");
        let basis = Mat4::from_cols_array_2d(&rotation.basis).transpose();
        assert!(
            basis.abs_diff_eq(Mat4::IDENTITY, 1e-4),
            "a joint at rest has the identity basis: {basis:?}"
        );
    }
}

#[test]
fn spring_preview_strand_lags_behind_a_turned_head() {
    let (bones, rest_locals, spring) = hanging_strand();
    let mut preview = SpringPreview::new(&spring, &bones, &rest_locals).unwrap();
    preview.reset(&rest_locals).unwrap();

    let mut turned = rest_locals.clone();
    let head = Mat4::from_quat(Quat::from_rotation_z(f32::to_radians(60.0)));
    turned[0] = head.transpose().to_cols_array_2d();
    let rotations = step_frames(&mut preview, &turned, 1);
    let root_angle = angle_degrees(rotations[0].rotation);
    assert!(
        root_angle > 10.0,
        "root joint should counter the head turn: {root_angle}"
    );

    let rigidly_turned_tip =
        Quat::from_rotation_z(f32::to_radians(60.0)) * Vec3::new(0.0, -0.4, 0.0);
    let tip_after_one_frame = Vec3::from(preview.tails().last().unwrap().position);
    assert!(
        (tip_after_one_frame - rigidly_turned_tip).length() > 0.1,
        "{tip_after_one_frame:?}"
    );

    step_frames(&mut preview, &turned, 300);
    let settled_tip = Vec3::from(preview.tails().last().unwrap().position);
    assert!(
        settled_tip.x > 0.1,
        "the strand should follow the head over time: {settled_tip:?}"
    );
}

#[test]
fn spring_preview_rejects_a_matrix_count_mismatch() {
    let (bones, rest_locals, spring) = hanging_strand();
    assert!(SpringPreview::new(&spring, &bones, &rest_locals[..2]).is_err());

    let mut preview = SpringPreview::new(&spring, &bones, &rest_locals).unwrap();
    assert!(preview.step(&rest_locals[..3], 1.0 / 30.0).is_err());
}
