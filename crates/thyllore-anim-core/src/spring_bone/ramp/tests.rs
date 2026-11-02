use cgmath::Vector3;

use super::*;
use crate::spring_bone::{JointForce, VrmJointForce};

fn make_chain(bone_ids: &[u32]) -> SpringChain {
    SpringChain {
        joints: bone_ids
            .iter()
            .map(|&bone_id| SpringJointParam {
                bone_id,
                force: vrm_force(1.0),
                drag_force: 0.5,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.1,
            })
            .collect(),
        ..Default::default()
    }
}

fn vrm_force(stiffness: f32) -> JointForce {
    JointForce::Vrm(VrmJointForce {
        stiffness,
        gravity_power: 1.0,
    })
}

fn stiffness_of(joint: &SpringJointParam) -> f32 {
    joint.force.magnitudes()[0]
}

fn stiffnesses_of(chain: &SpringChain) -> Vec<f32> {
    chain.joints.iter().map(stiffness_of).collect()
}

fn ramp(root_stiffness: f32, tip_stiffness: f32, shape: RampShape) -> ChainRamp {
    ChainRamp {
        root: SpringJointParam {
            force: vrm_force(root_stiffness),
            ..Default::default()
        },
        tip: SpringJointParam {
            force: vrm_force(tip_stiffness),
            ..Default::default()
        },
        shape,
        rigid_fraction: 0.0,
    }
}

fn build_head_ring_skeleton(
    chain_count: usize,
    chain_length: usize,
) -> (Skeleton, Vec<Vec<BoneId>>) {
    let mut skeleton = Skeleton::default();
    skeleton.add_bone("Head", None);
    let mut chain_bones: Vec<Vec<BoneId>> = Vec::new();
    for c in 0..chain_count {
        let mut prev: Option<u32> = Some(0);
        let mut bones: Vec<BoneId> = Vec::new();
        for j in 0..chain_length {
            let name = format!("chain{}_bone{}", c, j);
            let bone_id = skeleton.add_bone(&name, prev);
            bones.push(bone_id);
            prev = Some(bone_id);
        }
        chain_bones.push(bones);
    }
    (skeleton, chain_bones)
}

fn build_full_test_data(chain_count: usize, chain_length: usize) -> (Skeleton, SpringBoneSetup) {
    let (skeleton, chain_bones) = build_head_ring_skeleton(chain_count, chain_length);
    let mut setup = SpringBoneSetup::default();

    for (i, bones) in chain_bones.into_iter().enumerate() {
        let joints: Vec<SpringJointParam> = bones
            .iter()
            .map(|&bone_id| SpringJointParam {
                bone_id,
                ..Default::default()
            })
            .collect();
        setup.chains.push(SpringChain {
            id: i as u32,
            name: format!("chain{}", i),
            joints,
            ..Default::default()
        });
        setup.next_chain_id = i as u32 + 1;
    }
    (skeleton, setup)
}

#[test]
fn single_joint_is_root_value() {
    let chain = make_chain(&[0]);
    let ramp = ramp(0.8, 0.2, RampShape::Linear);
    let result = apply_ramp(&chain, &ramp).expect("one force model");

    assert_eq!(result.joints.len(), 1);
    assert!((stiffness_of(&result.joints[0]) - 0.8).abs() < 1e-6);
}

#[test]
fn four_joints_linear_interpolation() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let ramp = ramp(0.8, 0.2, RampShape::Linear);
    let result = apply_ramp(&chain, &ramp).expect("one force model");

    assert_eq!(result.joints.len(), 4);
    assert!((stiffness_of(&result.joints[0]) - 0.8).abs() < 1e-6);
    assert!((stiffness_of(&result.joints[1]) - 0.6).abs() < 1e-6);
    assert!((stiffness_of(&result.joints[2]) - 0.4).abs() < 1e-6);
    assert!((stiffness_of(&result.joints[3]) - 0.2).abs() < 1e-6);
}

#[test]
fn four_joints_ease_in_second_joint_closer_to_root() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let ramp_linear = ramp(0.8, 0.2, RampShape::Linear);
    let ramp_ease_in = ramp(0.8, 0.2, RampShape::EaseIn);

    let result_linear = apply_ramp(&chain, &ramp_linear).expect("one force model");
    let result_ease_in = apply_ramp(&chain, &ramp_ease_in).expect("one force model");
    assert!(stiffness_of(&result_ease_in.joints[1]) > stiffness_of(&result_linear.joints[1]));
}

#[test]
fn gravity_dir_is_root_for_all_joints() {
    let chain = make_chain(&[0, 1, 2]);
    let ramp = ChainRamp {
        root: SpringJointParam {
            force: vrm_force(0.8),
            gravity_dir: Vector3::new(1.0, 0.0, 0.0),
            ..Default::default()
        },
        tip: SpringJointParam {
            force: vrm_force(0.2),
            gravity_dir: Vector3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        shape: RampShape::Linear,
        rigid_fraction: 0.0,
    };
    let result = apply_ramp(&chain, &ramp).expect("one force model");

    for joint in &result.joints {
        assert_eq!(joint.gravity_dir, Vector3::new(1.0, 0.0, 0.0));
    }
}

#[test]
fn rigid_fraction_zero_matches_plain_shape() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let plain = ramp(0.8, 0.2, RampShape::EaseIn);
    let mut rigid = plain.clone();
    rigid.rigid_fraction = 0.0;

    let plain_result = apply_ramp(&chain, &plain).expect("one force model");
    let rigid_result = apply_ramp(&chain, &rigid).expect("one force model");
    assert_eq!(stiffnesses_of(&plain_result), stiffnesses_of(&rigid_result));
}

#[test]
fn rigid_fraction_half_holds_root_half_at_root_value() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let mut hem = ramp(0.8, 0.2, RampShape::Linear);
    hem.rigid_fraction = 0.5;
    let result = apply_ramp(&chain, &hem).expect("one force model");

    assert!((stiffness_of(&result.joints[0]) - 0.8).abs() < 1e-6);
    assert!((stiffness_of(&result.joints[1]) - 0.8).abs() < 1e-6);
    assert!((stiffness_of(&result.joints[2]) - 0.6).abs() < 1e-6);
    assert!((stiffness_of(&result.joints[3]) - 0.2).abs() < 1e-6);
}

#[test]
fn rigid_fraction_one_gives_every_joint_the_root_value() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let mut hem = ramp(0.8, 0.2, RampShape::EaseOut);
    hem.rigid_fraction = 1.0;
    let result = apply_ramp(&chain, &hem).expect("one force model");

    for joint in &result.joints {
        assert!((stiffness_of(joint) - 0.8).abs() < 1e-6);
    }
}

#[test]
fn rigid_fraction_outside_the_unit_range_is_clamped() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let mut below = ramp(0.8, 0.2, RampShape::Linear);
    below.rigid_fraction = -0.5;
    let mut above = ramp(0.8, 0.2, RampShape::Linear);
    above.rigid_fraction = 1.5;

    let plain = apply_ramp(&chain, &ramp(0.8, 0.2, RampShape::Linear)).expect("one force model");
    let below_result = apply_ramp(&chain, &below).expect("one force model");
    assert_eq!(stiffnesses_of(&below_result), stiffnesses_of(&plain));
    let all_root = apply_ramp(&chain, &above).expect("one force model");
    for joint in &all_root.joints {
        assert!((stiffness_of(joint) - 0.8).abs() < 1e-6);
    }
}

#[test]
fn bone_ids_preserved_from_input_chain() {
    let chain = make_chain(&[5, 12, 7]);
    let ramp = ramp(0.8, 0.2, RampShape::Linear);
    let result = apply_ramp(&chain, &ramp).expect("one force model");

    assert_eq!(result.joints[0].bone_id, 5);
    assert_eq!(result.joints[1].bone_id, 12);
    assert_eq!(result.joints[2].bone_id, 7);
}

#[test]
fn ramp_with_ends_of_different_models_is_rejected() {
    let chain = make_chain(&[0, 1]);
    let mut mixed = ramp(0.8, 0.2, RampShape::Linear);
    mixed.tip.force = JointForce::default_for(SpringForceModel::RestRelative);

    assert_eq!(
        apply_ramp(&chain, &mixed).err(),
        Some(RampForceMismatch::EndsDiffer {
            root: SpringForceModel::Vrm,
            tip: SpringForceModel::RestRelative,
        })
    );
}

#[test]
fn ramp_of_another_model_than_the_chain_is_rejected() {
    let chain = make_chain(&[0, 1]);

    assert_eq!(
        apply_ramp(&chain, &ChainRamp::default()).err(),
        Some(RampForceMismatch::JointDiffers {
            ramp: SpringForceModel::RestRelative,
            joint: SpringForceModel::Vrm,
        })
    );
}

#[test]
fn sibling_chains_returns_all_six() {
    let (skeleton, setup) = build_full_test_data(6, 4);

    let result = chains_in_scope(&setup, &skeleton, 0, RampScope::SiblingChains);
    assert_eq!(result.len(), 6);
    for i in 0..6u32 {
        assert!(result.contains(&i), "chain {} should be in scope", i);
    }
}

#[test]
fn chains_under_head_are_all_six_and_a_joint_bone_has_none() {
    let (skeleton, setup) = build_full_test_data(6, 4);

    let under_head = chains_under_bone(&setup, &skeleton, 0);
    assert_eq!(under_head, (0..6u32).collect::<Vec<_>>());
    assert!(chains_under_bone(&setup, &skeleton, 1).is_empty());
}

#[test]
fn joint_count_sums_only_the_listed_chains() {
    let (_, setup) = build_full_test_data(6, 4);

    assert_eq!(joint_count_of(&setup, &[0, 2]), 8);
    assert_eq!(joint_count_of(&setup, &[]), 0);
}

#[test]
fn sibling_chains_empty_for_root_chain_without_parent() {
    let mut skeleton = Skeleton::default();
    skeleton.add_bone("Hips", None);
    skeleton.add_bone("Spine", Some(0));

    let mut setup = SpringBoneSetup::default();
    setup.chains.push(SpringChain {
        id: 0,
        name: "root_chain".to_string(),
        joints: vec![SpringJointParam {
            bone_id: 0,
            ..Default::default()
        }],
        ..Default::default()
    });
    setup.next_chain_id = 1;
    setup.chains.push(SpringChain {
        id: 1,
        name: "spine_chain".to_string(),
        joints: vec![SpringJointParam {
            bone_id: 1,
            ..Default::default()
        }],
        ..Default::default()
    });
    setup.next_chain_id = 2;

    let result = chains_in_scope(&setup, &skeleton, 0, RampScope::SiblingChains);
    assert!(
        result.is_empty(),
        "root chain with no parent should return empty sibling scope"
    );
}

#[test]
fn this_chain_returns_only_self() {
    let (skeleton, setup) = build_full_test_data(6, 4);

    let result = chains_in_scope(&setup, &skeleton, 3, RampScope::ThisChain);
    assert_eq!(result, vec![3]);
}

#[test]
fn all_chains_returns_all() {
    let (skeleton, setup) = build_full_test_data(6, 4);

    let result = chains_in_scope(&setup, &skeleton, 2, RampScope::AllChains);
    assert_eq!(result.len(), 6);
    for i in 0..6u32 {
        assert!(result.contains(&i), "chain {} should be in scope", i);
    }
}

#[test]
fn sibling_chains_excludes_different_parent() {
    let mut skeleton = Skeleton::default();
    skeleton.add_bone("Head", None);
    skeleton.add_bone("Neck", Some(0));

    for c in 0..3u32 {
        let name = format!("head_chain{}_bone0", c);
        skeleton.add_bone(&name, Some(0));
    }
    skeleton.add_bone("neck_chain0", Some(1));

    let mut setup = SpringBoneSetup::default();
    for i in 0..3u32 {
        setup.chains.push(SpringChain {
            id: i,
            name: format!("head_chain{}", i),
            joints: vec![SpringJointParam {
                bone_id: i + 2,
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    setup.next_chain_id = 3;
    setup.chains.push(SpringChain {
        id: 3,
        name: "neck_chain".to_string(),
        joints: vec![SpringJointParam {
            bone_id: 5,
            ..Default::default()
        }],
        ..Default::default()
    });
    setup.next_chain_id = 4;

    let result = chains_in_scope(&setup, &skeleton, 0, RampScope::SiblingChains);
    assert_eq!(result.len(), 3);
    assert!(result.contains(&0));
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(!result.contains(&3), "neck chain should not be in scope");
}

#[test]
fn missing_chain_returns_empty() {
    let (skeleton, setup) = build_full_test_data(6, 4);

    let result = chains_in_scope(&setup, &skeleton, 99, RampScope::SiblingChains);
    assert!(result.is_empty());
}

#[test]
fn set_end_root() {
    let mut ramp: ChainRamp = Default::default();
    let joint = SpringJointParam {
        bone_id: 5,
        force: vrm_force(0.9),
        ..Default::default()
    };
    ramp.set_end(RampEnd::Root, joint);
    assert_eq!(ramp.root.bone_id, 5);
    assert_eq!(stiffness_of(&ramp.root), 0.9);
}

#[test]
fn set_end_tip() {
    let mut ramp: ChainRamp = Default::default();
    let joint = SpringJointParam {
        bone_id: 7,
        force: vrm_force(0.1),
        ..Default::default()
    };
    ramp.set_end(RampEnd::Tip, joint);
    assert_eq!(ramp.tip.bone_id, 7);
    assert_eq!(stiffness_of(&ramp.tip), 0.1);
}

#[test]
fn load_ends_normal_chain() {
    let mut ramp: ChainRamp = Default::default();
    let chain = SpringChain {
        id: 0,
        name: "test".to_string(),
        joints: vec![
            SpringJointParam {
                bone_id: 1,
                force: vrm_force(0.8),
                ..Default::default()
            },
            SpringJointParam {
                bone_id: 2,
                force: vrm_force(0.5),
                ..Default::default()
            },
            SpringJointParam {
                bone_id: 3,
                force: vrm_force(0.2),
                ..Default::default()
            },
        ],
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        tail_bone_id: None,
        enabled: true,
        origin: Default::default(),
    };
    ramp.load_ends(&chain);
    assert_eq!(ramp.root.bone_id, 1);
    assert_eq!(stiffness_of(&ramp.root), 0.8);
    assert_eq!(ramp.tip.bone_id, 3);
    assert_eq!(stiffness_of(&ramp.tip), 0.2);
}

#[test]
fn load_ends_empty_chain_is_noop() {
    let mut ramp: ChainRamp = Default::default();
    ramp.root = SpringJointParam {
        bone_id: 99,
        force: vrm_force(0.5),
        ..Default::default()
    };
    let chain = SpringChain {
        id: 0,
        name: "empty".to_string(),
        joints: Vec::new(),
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        tail_bone_id: None,
        enabled: true,
        origin: Default::default(),
    };
    ramp.load_ends(&chain);
    assert_eq!(ramp.root.bone_id, 99);
    assert_eq!(stiffness_of(&ramp.root), 0.5);
}

#[test]
fn chain_ramp_ui_params_names() {
    let names: Vec<&str> = CHAIN_RAMP_UI_PARAMS
        .iter()
        .map(|p| p.name.as_ref())
        .collect();
    assert_eq!(
        names,
        [
            "root_drag_force",
            "root_gravity_dir",
            "root_hit_radius",
            "tip_drag_force",
            "tip_gravity_dir",
            "tip_hit_radius",
            "rigid_fraction"
        ]
    );
}

#[test]
fn targets_without_skeleton_are_the_chain_alone() {
    let (_, setup) = build_full_test_data(6, 3);
    let targets = RampTargets::in_scope(&setup, None, 1, RampScope::SiblingChains);
    assert_eq!(targets.chains, vec![1]);
    assert_eq!(targets.joint_count, 3);
}

#[test]
fn targets_in_sibling_scope_reach_every_chain_under_the_head() {
    let (skeleton, setup) = build_full_test_data(6, 3);
    let siblings = RampTargets::in_scope(&setup, Some(&skeleton), 1, RampScope::SiblingChains);
    assert_eq!(siblings.chains.len(), 6);
    assert_eq!(siblings.joint_count, 18);
    let own = RampTargets::in_scope(&setup, Some(&skeleton), 1, RampScope::ThisChain);
    assert_eq!(own.chains, vec![1]);
}

#[test]
fn targets_under_a_bone_exist_only_above_chain_roots() {
    let (skeleton, setup) = build_full_test_data(6, 3);
    let head = RampTargets::under_bone(&setup, Some(&skeleton), 0).expect("head has chains");
    assert_eq!(head.chains.len(), 6);
    assert!(RampTargets::under_bone(&setup, Some(&skeleton), 1).is_none());
    assert!(RampTargets::under_bone(&setup, None, 0).is_none());
}
