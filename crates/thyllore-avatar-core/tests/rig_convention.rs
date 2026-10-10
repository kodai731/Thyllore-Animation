#![cfg(test)]
mod support;

use support::rig_convention::{rig_convention, BoneAxisRule, ContainerNode};
use support::rig_names::CONVENTIONS;

#[test]
fn all_conventions_resolve() {
    for id in CONVENTIONS {
        let conv = rig_convention(id);
        assert_eq!(conv.id, id);
        assert!(conv.unit_scale_factor > 0.0);
        assert!(conv.body_scale > 0.0);
    }
}

#[test]
fn unknown_id_panics() {
    let result = std::panic::catch_unwind(|| rig_convention("unknown"));
    assert!(result.is_err());
}

#[test]
fn blender_values() {
    let conv = rig_convention("blender");
    assert_eq!(conv.id, "blender");
    assert!(conv.character_to_file.is_empty());
    assert_eq!(conv.up_axis, 1);
    assert!((conv.unit_scale_factor - 100.0).abs() < 1e-9);
    match conv.bone_axis {
        BoneAxisRule::Along(axis) => {
            assert!((axis[0] - 0.0).abs() < 1e-9);
            assert!((axis[1] - 1.0).abs() < 1e-9);
            assert!((axis[2] - 0.0).abs() < 1e-9);
        }
        _ => panic!("expected Along"),
    }
    assert!((conv.roll_degrees - 0.0).abs() < 1e-9);
    assert!(!conv.uses_pre_rotation);
    assert!((conv.body_scale - 1.0).abs() < 1e-9);
    assert!((conv.arm_drop_degrees - 0.0).abs() < 1e-9);
    match conv.container {
        ContainerNode::Armature { rotation_x_degrees } => {
            assert!((rotation_x_degrees - (-90.0)).abs() < 1e-9);
        }
        _ => panic!("expected Armature"),
    }
    assert!(conv.leaf_end_bones);
}

#[test]
fn maya_values() {
    let conv = rig_convention("maya");
    assert_eq!(conv.id, "maya");
    assert!(conv.character_to_file.is_empty());
    assert_eq!(conv.up_axis, 1);
    assert!((conv.unit_scale_factor - 1.0).abs() < 1e-9);
    match conv.bone_axis {
        BoneAxisRule::Along(axis) => {
            assert!((axis[0] - 1.0).abs() < 1e-9);
            assert!((axis[1] - 0.0).abs() < 1e-9);
            assert!((axis[2] - 0.0).abs() < 1e-9);
        }
        _ => panic!("expected Along"),
    }
    assert!(conv.uses_pre_rotation);
    match conv.container {
        ContainerNode::None => {}
        _ => panic!("expected None"),
    }
    assert!(!conv.leaf_end_bones);
}

#[test]
fn max_biped_values() {
    let conv = rig_convention("max_biped");
    assert_eq!(conv.id, "max_biped");
    assert_eq!(conv.character_to_file.len(), 1);
    assert_eq!(conv.up_axis, 2);
    assert!((conv.unit_scale_factor - 2.54).abs() < 1e-9);
    assert!((conv.roll_degrees - 90.0).abs() < 1e-9);
    match conv.container {
        ContainerNode::BipedRoot {
            rotation_up_degrees,
        } => {
            assert!((rotation_up_degrees - 90.0).abs() < 1e-9);
        }
        _ => panic!("expected BipedRoot"),
    }
}

#[test]
fn mixamo_values() {
    let conv = rig_convention("mixamo");
    assert_eq!(conv.id, "mixamo");
    assert!(conv.character_to_file.is_empty());
    assert_eq!(conv.up_axis, 1);
    assert!((conv.unit_scale_factor - 1.0).abs() < 1e-9);
    match conv.container {
        ContainerNode::None => {}
        _ => panic!("expected None"),
    }
}

#[test]
fn unreal_values() {
    let conv = rig_convention("unreal");
    assert_eq!(conv.id, "unreal");
    assert_eq!(conv.character_to_file.len(), 2);
    assert_eq!(conv.up_axis, 2);
    match conv.container {
        ContainerNode::RootBone => {}
        _ => panic!("expected RootBone"),
    }
}

#[test]
fn vrm_normalized_values() {
    let conv = rig_convention("vrm_normalized");
    assert_eq!(conv.id, "vrm_normalized");
    assert!(conv.character_to_file.is_empty());
    assert_eq!(conv.up_axis, 1);
    assert!((conv.unit_scale_factor - 100.0).abs() < 1e-9);
    match conv.bone_axis {
        BoneAxisRule::WorldAligned => {}
        _ => panic!("expected WorldAligned"),
    }
}

#[test]
fn blender_apose_values() {
    let conv = rig_convention("blender_apose");
    assert_eq!(conv.id, "blender_apose");
    assert!((conv.arm_drop_degrees - 45.0).abs() < 1e-9);
    assert!(conv.leaf_end_bones);
}

#[test]
fn adversarial_values() {
    let conv = rig_convention("adversarial");
    assert_eq!(conv.id, "adversarial");
    assert_eq!(conv.character_to_file.len(), 2);
    assert!((conv.body_scale - 0.5).abs() < 1e-9);
    match conv.bone_axis {
        BoneAxisRule::Random { seed } => {
            assert_eq!(seed, 7);
        }
        _ => panic!("expected Random"),
    }
}
