use thyllore_anim_core::editable::components::clip::{ClipSpace, EditableAnimationClip};
use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;

use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

fn role_index(name: &str) -> usize {
    HumanoidRole::ALL
        .iter()
        .position(|r| format!("{r:?}") == name)
        .unwrap_or_else(|| panic!("role not found in HumanoidRole::ALL: {name}"))
}

pub fn role_clip(
    name: &str,
    duration: f32,
    rotation_keys: &[(f32, &str, [f32; 3])],
    hips_keys: &[(f32, [f32; 3])],
) -> EditableAnimationClip {
    let mut clip = EditableAnimationClip::new(1, name.to_string());
    clip.duration = duration;
    clip.space = ClipSpace::HumanoidRole;

    let mut roles: Vec<&str> = Vec::new();
    for (_, role_name, _) in rotation_keys {
        if !roles.contains(role_name) {
            roles.push(*role_name);
        }
    }

    if !hips_keys.is_empty() && !roles.contains(&"Hips") {
        roles.push("Hips");
    }

    for &role_name in &roles {
        let bone_id = role_index(role_name) as u32;
        let track = clip.add_track(bone_id, role_name.to_string());

        for (time, rname, xyz) in rotation_keys {
            if *rname == role_name {
                curve_add_keyframe(&mut track.rotation_x, *time, xyz[0]);
                curve_add_keyframe(&mut track.rotation_y, *time, xyz[1]);
                curve_add_keyframe(&mut track.rotation_z, *time, xyz[2]);
            }
        }

        if role_name == "Hips" {
            for (time, xyz) in hips_keys {
                curve_add_keyframe(&mut track.translation_x, *time, xyz[0]);
                curve_add_keyframe(&mut track.translation_y, *time, xyz[1]);
                curve_add_keyframe(&mut track.translation_z, *time, xyz[2]);
            }
        }
    }

    clip
}

pub fn wave() -> EditableAnimationClip {
    role_clip(
        "wave_right_hand",
        2.8,
        &[
            (0.0, "RightUpperArm", [0.0, 0.0, 20.0]),
            (0.0, "RightLowerArm", [0.0, 0.0, 90.0]),
            (0.0, "Head", [0.0, 10.0, 0.0]),
            (0.4, "RightLowerArm", [0.0, 0.0, 110.0]),
            (0.4, "RightHand", [0.0, 0.0, 20.0]),
            (0.8, "RightLowerArm", [0.0, 0.0, 70.0]),
            (0.8, "RightHand", [0.0, 0.0, -20.0]),
            (2.8, "RightUpperArm", [0.0, 0.0, 0.0]),
            (2.8, "RightLowerArm", [0.0, 0.0, 0.0]),
            (2.8, "RightHand", [0.0, 0.0, 0.0]),
            (2.8, "Head", [0.0, 0.0, 0.0]),
        ],
        &[],
    )
}

pub fn bow() -> EditableAnimationClip {
    role_clip(
        "bow",
        2.0,
        &[
            (0.0, "Spine", [0.0, 0.0, 0.0]),
            (0.0, "Chest", [0.0, 0.0, 0.0]),
            (0.0, "Head", [0.0, 0.0, 0.0]),
            (1.0, "Spine", [20.0, 0.0, 0.0]),
            (1.0, "Chest", [20.0, 0.0, 0.0]),
            (1.0, "Head", [5.0, 0.0, 0.0]),
            (2.0, "Spine", [0.0, 0.0, 0.0]),
            (2.0, "Chest", [0.0, 0.0, 0.0]),
            (2.0, "Head", [0.0, 0.0, 0.0]),
        ],
        &[
            (0.0, [0.0, 0.0, 0.0]),
            (1.0, [0.0, 0.0, -0.05]),
            (2.0, [0.0, 0.0, 0.0]),
        ],
    )
}

pub fn hands_on_hips_tilt() -> EditableAnimationClip {
    role_clip(
        "hands_on_hips_tilt",
        2.0,
        &[
            (0.0, "LeftUpperArm", [0.0, 0.0, 0.0]),
            (0.0, "RightUpperArm", [0.0, 0.0, 0.0]),
            (0.0, "LeftLowerArm", [0.0, 0.0, 0.0]),
            (0.0, "RightLowerArm", [0.0, 0.0, 0.0]),
            (0.0, "Head", [0.0, 0.0, 0.0]),
            (0.8, "RightUpperArm", [0.0, 15.0, -50.0]),
            (0.8, "LeftUpperArm", [0.0, -15.0, 50.0]),
            (0.8, "RightLowerArm", [0.0, -30.0, -75.0]),
            (0.8, "LeftLowerArm", [0.0, 30.0, 75.0]),
            (0.8, "Head", [0.0, 0.0, 0.0]),
            (1.4, "Head", [0.0, 0.0, -15.0]),
            (2.0, "RightUpperArm", [0.0, 15.0, -50.0]),
            (2.0, "LeftUpperArm", [0.0, -15.0, 50.0]),
            (2.0, "RightLowerArm", [0.0, -30.0, -75.0]),
            (2.0, "LeftLowerArm", [0.0, 30.0, 75.0]),
            (2.0, "Head", [0.0, 0.0, -15.0]),
        ],
        &[],
    )
}

pub fn mixed() -> EditableAnimationClip {
    role_clip(
        "mixed_axes",
        2.0,
        &[
            (0.0, "Spine", [0.0, 0.0, 0.0]),
            (0.0, "LeftUpperArm", [0.0, 0.0, 0.0]),
            (0.0, "RightUpperArm", [0.0, 0.0, 0.0]),
            (0.8, "Spine", [10.0, 25.0, -5.0]),
            (0.8, "Chest", [-15.0, 10.0, 0.0]),
            (0.8, "Neck", [5.0, -20.0, 0.0]),
            (0.8, "Head", [20.0, 15.0, -10.0]),
            (0.8, "RightShoulder", [0.0, 0.0, 15.0]),
            (0.8, "LeftShoulder", [0.0, 0.0, -15.0]),
            (0.8, "RightUpperArm", [20.0, -35.0, 40.0]),
            (0.8, "LeftUpperArm", [-30.0, 45.0, -60.0]),
            (0.8, "RightLowerArm", [0.0, -70.0, 10.0]),
            (0.8, "LeftLowerArm", [0.0, 80.0, 0.0]),
            (0.8, "RightHand", [10.0, 0.0, -25.0]),
            (0.8, "LeftHand", [-10.0, 20.0, 30.0]),
            (0.8, "RightUpperLeg", [-45.0, 10.0, 5.0]),
            (0.8, "LeftUpperLeg", [30.0, -10.0, -5.0]),
            (0.8, "RightLowerLeg", [60.0, 0.0, 0.0]),
            (0.8, "LeftLowerLeg", [20.0, 0.0, 0.0]),
            (0.8, "RightFoot", [-20.0, 5.0, 0.0]),
            (0.8, "LeftFoot", [15.0, -5.0, 0.0]),
            (2.0, "Spine", [-5.0, -30.0, 10.0]),
            (2.0, "RightUpperArm", [-60.0, 20.0, -15.0]),
            (2.0, "LeftUpperArm", [70.0, -25.0, 15.0]),
            (2.0, "RightUpperLeg", [20.0, 0.0, 0.0]),
        ],
        &[(0.8, [0.05, -0.2, 0.1]), (2.0, [0.0, 0.0, 0.0])],
    )
}
