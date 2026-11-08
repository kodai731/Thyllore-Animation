use glam::{Quat, Vec3};
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring;

use super::*;
use crate::avatar_setup_bridge::BoneSpec;
use crate::blender_space::column_major;
use crate::spring_edit_bridge::{use_existing_bones_as_springs, UseExistingBonesAsSpringsRequest};

const IDENTITY_ROWS: MatrixRows = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn bone(name: &str, parent: Option<usize>, rest_position: [f32; 3]) -> BoneSpec {
    BoneSpec {
        name: name.to_string(),
        parent,
        rest_position,
        rest_frame: None,
    }
}

/// Head at the origin with a three-bone strand hanging straight down Blender's -Z axis, the
/// engine's -Y once the armature frame turns the axes.
fn hanging_strand() -> (SpringPreviewRequest, Vec<MatrixRows>) {
    let bones = vec![
        bone("Head", None, [0.0, 0.0, 0.0]),
        bone("hair_a", Some(0), [0.0, -0.1, 0.0]),
        bone("hair_a.001", Some(1), [0.0, -0.2, 0.0]),
        bone("hair_a.002", Some(2), [0.0, -0.3, 0.0]),
    ];
    let rest_matrices: Vec<MatrixRows> = (0..4)
        .map(|index| translation_rows(Vec3::new(0.0, 0.0, -0.1 * index as f32)))
        .collect();
    let spring = use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: bones.clone(),
        root_bone: "Head".to_string(),
        min_joints: 2,
    })
    .unwrap();
    let request = SpringPreviewRequest {
        spring,
        bones,
        armature_world: IDENTITY_ROWS,
        rest_matrices: rest_matrices.clone(),
        scale_length: 1.0,
    };
    (request, rest_matrices)
}

fn pose(pose_matrices: &[MatrixRows]) -> PoseRequest {
    PoseRequest {
        armature_world: IDENTITY_ROWS,
        pose_matrices: pose_matrices.to_vec(),
    }
}

fn step_frames(
    preview: &mut SpringPreview,
    pose_matrices: &[MatrixRows],
    frames: usize,
) -> Vec<JointPose> {
    let mut last = Vec::new();
    for _ in 0..frames {
        last = preview
            .step(&StepRequest {
                armature_world: IDENTITY_ROWS,
                pose_matrices: pose_matrices.to_vec(),
                dt: 1.0 / 30.0,
            })
            .unwrap();
    }
    last
}

fn angle_degrees(rotation: [f32; 4]) -> f32 {
    2.0 * rotation[0].clamp(-1.0, 1.0).acos().to_degrees()
}

#[test]
fn spring_preview_strand_at_rest_under_gravity_stays_still() {
    let (request, rest_matrices) = hanging_strand();
    let mut preview = SpringPreview::new(&request).unwrap();

    let rotations = step_frames(&mut preview, &rest_matrices, 30);

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
    let (request, rest_matrices) = hanging_strand();
    let mut preview = SpringPreview::new(&request).unwrap();
    preview.reset(&pose(&rest_matrices)).unwrap();

    let head = Mat4::from_quat(Quat::from_rotation_y(f32::to_radians(-60.0)));
    let turned: Vec<MatrixRows> = rest_matrices
        .iter()
        .map(|matrix| rows(head * column_major(matrix)))
        .collect();
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
    let (mut request, rest_matrices) = hanging_strand();
    request.rest_matrices.truncate(2);
    assert!(SpringPreview::new(&request).is_err());

    request.rest_matrices = rest_matrices.clone();
    let mut preview = SpringPreview::new(&request).unwrap();
    assert!(preview
        .step(&StepRequest {
            armature_world: IDENTITY_ROWS,
            pose_matrices: rest_matrices[..3].to_vec(),
            dt: 1.0 / 30.0,
        })
        .is_err());
}
