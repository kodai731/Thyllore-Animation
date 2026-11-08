use glam::Vec3;

use super::*;
use crate::blender_space::column_major;

const HIPS_SWAY: &str = include_str!("../../../../assets/templates/hips_sway.anim.ron");
const IDENTITY_FRAME: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const IDENTITY_ROWS: MatrixRows = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Every bone's `matrix_local` as a translation to its rest head read back to Blender axes.
fn rest_matrices(bones: &[BoneSpec]) -> Vec<MatrixRows> {
    bones
        .iter()
        .map(|bone| {
            let [x, y, z] = bone.rest_position;
            rows(Mat4::from_translation(Vec3::new(x, -z, y)))
        })
        .collect()
}

fn request(bones: Vec<BoneSpec>) -> BakeClipFileRequest {
    BakeClipFileRequest {
        rest_matrices: rest_matrices(&bones),
        bones,
        clip_ron: HIPS_SWAY.to_string(),
        fps: 2,
        armature_world: IDENTITY_ROWS,
        scale_length: 1.0,
    }
}

fn bone(name: &str, parent: Option<usize>, rest_position: [f32; 3]) -> BoneSpec {
    BoneSpec {
        name: name.to_string(),
        parent,
        rest_position,
        rest_frame: Some(IDENTITY_FRAME),
    }
}

fn tpose_bones(prefix: &str) -> Vec<BoneSpec> {
    let named = |role: &str| format!("{prefix}{role}");
    vec![
        bone(&named("Hips"), None, [0.0, 1.0, 0.0]),
        bone(&named("Spine"), Some(0), [0.0, 1.1, 0.0]),
        bone(&named("Chest"), Some(1), [0.0, 1.3, 0.0]),
        bone(&named("Neck"), Some(2), [0.0, 1.5, 0.0]),
        bone(&named("Head"), Some(3), [0.0, 1.6, 0.0]),
        bone(&named("LeftUpperArm"), Some(2), [0.18, 1.45, 0.0]),
        bone(&named("RightUpperArm"), Some(2), [-0.18, 1.45, 0.0]),
        bone(&named("LeftUpperLeg"), Some(0), [0.1, 0.95, 0.0]),
        bone(&named("LeftLowerLeg"), Some(7), [0.1, 0.5, 0.0]),
        bone(&named("LeftFoot"), Some(8), [0.1, 0.0, 0.0]),
        bone(&named("RightUpperLeg"), Some(0), [-0.1, 0.95, 0.0]),
        bone(&named("RightLowerLeg"), Some(10), [-0.1, 0.5, 0.0]),
        bone(&named("RightFoot"), Some(11), [-0.1, 0.0, 0.0]),
    ]
}

#[test]
fn clip_bake_bridge_sways_hips_sideways_by_the_hips_height() {
    let baked = bake_clip_file(&request(tpose_bones(""))).unwrap();

    assert_eq!(baked.name, "hips_sway");
    assert_eq!(baked.hips_bone, "Hips");
    assert_eq!(baked.frame_times, vec![0.0, 0.5, 1.0, 1.5, 2.0]);
    let sideways: Vec<f32> = baked.hips_offsets.iter().map(|v| v[0]).collect();
    assert!((sideways[1].abs() - 0.15).abs() < 1e-5, "{sideways:?}");
    assert!((sideways[3] + sideways[1]).abs() < 1e-5, "{sideways:?}");
    assert!(
        sideways[0].abs() < 1e-5 && sideways[4].abs() < 1e-5,
        "{sideways:?}"
    );
    for offset in &baked.hips_offsets {
        assert!(
            offset[1].abs() < 1e-5 && offset[2].abs() < 1e-5,
            "{offset:?}"
        );
    }
}

#[test]
fn clip_bake_bridge_binds_role_tracks_to_prefixed_bones() {
    let baked = bake_clip_file(&request(tpose_bones("mixamorig:"))).unwrap();

    assert_eq!(baked.hips_bone, "mixamorig:Hips");
    assert!((baked.hips_offsets[1][0].abs() - 0.15).abs() < 1e-5);
    let tracked: Vec<&str> = baked
        .bone_rotations
        .iter()
        .map(|t| t.bone.as_str())
        .collect();
    assert!(tracked.contains(&"mixamorig:Head"), "{tracked:?}");
    for track in &baked.bone_rotations {
        assert_eq!(track.rotations.len(), 5, "{}", track.bone);
    }
}

#[test]
fn clip_bake_bridge_rejects_a_rig_without_feet() {
    let bones: Vec<BoneSpec> = tpose_bones("").into_iter().take(7).collect();

    let error = bake_clip_file(&request(bones)).unwrap_err();

    assert!(error.to_string().contains("feet"), "{error}");
}

#[test]
fn clip_bake_bridge_rejects_a_bone_without_rest_frame() {
    let mut bones = tpose_bones("");
    bones[0].rest_frame = None;

    let error = bake_clip_file(&request(bones)).unwrap_err();

    assert!(error.to_string().contains("rest_frame"), "{error}");
}

#[test]
fn clip_bake_bridge_writes_the_hips_sway_as_a_bone_local_location() {
    let baked = bake_clip_file(&request(tpose_bones(""))).unwrap();

    let sideways: Vec<f32> = baked.hips_locations.iter().map(|v| v[0]).collect();
    assert!((sideways[1].abs() - 0.15).abs() < 1e-5, "{sideways:?}");
    for location in &baked.hips_locations {
        assert!(
            location[1].abs() < 1e-5 && location[2].abs() < 1e-5,
            "{location:?}"
        );
    }
}

#[test]
fn clip_bake_bridge_basis_of_an_unrotated_bone_is_the_identity() {
    let baked = bake_clip_file(&request(tpose_bones(""))).unwrap();

    let head = baked
        .bone_rotations
        .iter()
        .find(|track| track.bone == "Head")
        .unwrap();
    assert_eq!(head.bases.len(), baked.frame_times.len());
    for basis in &head.bases {
        let basis = column_major(basis);
        assert!(basis.abs_diff_eq(Mat4::IDENTITY, 1e-4), "{basis:?}");
    }
}
