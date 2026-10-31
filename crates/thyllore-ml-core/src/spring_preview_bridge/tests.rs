use cgmath::{Deg, InnerSpace, Quaternion, Rotation3, Vector3};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring;

use super::*;
use crate::spring_edit_bridge::add_chains_under_bone;

fn bone(name: &str, parent: Option<usize>, rest_position: [f32; 3]) -> BoneInput {
    BoneInput {
        name: name.to_string(),
        parent,
        rest_position,
    }
}

/// Head at the origin with a three-bone strand hanging straight down the -Y axis.
fn hanging_strand() -> (Vec<BoneInput>, Vec<MatrixRows>, SidecarSpring) {
    let bones = vec![
        bone("Head", None, [0.0, 0.0, 0.0]),
        bone("hair_a", Some(0), [0.0, -0.1, 0.0]),
        bone("hair_a.001", Some(1), [0.0, -0.2, 0.0]),
        bone("hair_a.002", Some(2), [0.0, -0.3, 0.0]),
    ];
    let rest_locals = vec![
        translation_rows(Vector3::new(0.0, 0.0, 0.0)),
        translation_rows(Vector3::new(0.0, -0.1, 0.0)),
        translation_rows(Vector3::new(0.0, -0.1, 0.0)),
        translation_rows(Vector3::new(0.0, -0.1, 0.0)),
    ];
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 2).unwrap();
    (bones, rest_locals, spring)
}

fn step_frames(
    preview: &mut SpringPreview,
    locals: &[MatrixRows],
    frames: usize,
) -> Vec<JointRotation> {
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
    }
}

#[test]
fn spring_preview_strand_lags_behind_a_turned_head() {
    let (bones, rest_locals, spring) = hanging_strand();
    let mut preview = SpringPreview::new(&spring, &bones, &rest_locals).unwrap();
    preview.reset(&rest_locals).unwrap();

    let mut turned = rest_locals.clone();
    let head = cgmath::Matrix4::from(Quaternion::from_angle_z(Deg(60.0)));
    turned[0] = [
        [head.x.x, head.y.x, head.z.x, 0.0],
        [head.x.y, head.y.y, head.z.y, 0.0],
        [head.x.z, head.y.z, head.z.z, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let rotations = step_frames(&mut preview, &turned, 1);
    let root_angle = angle_degrees(rotations[0].rotation);
    assert!(
        root_angle > 10.0,
        "root joint should counter the head turn: {root_angle}"
    );

    let rigidly_turned_tip = Quaternion::from_angle_z(Deg(60.0)) * Vector3::new(0.0, -0.4, 0.0);
    let tip_after_one_frame = Vector3::from(preview.tails().last().unwrap().position);
    assert!(
        (tip_after_one_frame - rigidly_turned_tip).magnitude() > 0.1,
        "{tip_after_one_frame:?}"
    );

    step_frames(&mut preview, &turned, 300);
    let settled_tip = Vector3::from(preview.tails().last().unwrap().position);
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
