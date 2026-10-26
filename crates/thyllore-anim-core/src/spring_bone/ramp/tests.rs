use cgmath::Vector3;

use super::*;

fn make_chain(bone_ids: &[u32]) -> SpringChain {
    SpringChain {
        joints: bone_ids
            .iter()
            .map(|&bone_id| SpringJointParam {
                bone_id,
                stiffness: 1.0,
                drag_force: 0.5,
                gravity_power: 1.0,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.1,
            })
            .collect(),
        ..Default::default()
    }
}

fn ramp(root_stiffness: f32, tip_stiffness: f32, shape: RampShape) -> ChainRamp {
    ChainRamp {
        root: SpringJointParam {
            stiffness: root_stiffness,
            ..Default::default()
        },
        tip: SpringJointParam {
            stiffness: tip_stiffness,
            ..Default::default()
        },
        shape,
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
    let result = apply_ramp(&chain, &ramp);

    assert_eq!(result.joints.len(), 1);
    assert!((result.joints[0].stiffness - 0.8).abs() < 1e-6);
}

#[test]
fn four_joints_linear_interpolation() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let ramp = ramp(0.8, 0.2, RampShape::Linear);
    let result = apply_ramp(&chain, &ramp);

    assert_eq!(result.joints.len(), 4);
    assert!((result.joints[0].stiffness - 0.8).abs() < 1e-6);
    assert!((result.joints[1].stiffness - 0.6).abs() < 1e-6);
    assert!((result.joints[2].stiffness - 0.4).abs() < 1e-6);
    assert!((result.joints[3].stiffness - 0.2).abs() < 1e-6);
}

#[test]
fn four_joints_ease_in_second_joint_closer_to_root() {
    let chain = make_chain(&[0, 1, 2, 3]);
    let ramp_linear = ramp(0.8, 0.2, RampShape::Linear);
    let ramp_ease_in = ramp(0.8, 0.2, RampShape::EaseIn);

    let result_linear = apply_ramp(&chain, &ramp_linear);
    let result_ease_in = apply_ramp(&chain, &ramp_ease_in);
    assert!(result_ease_in.joints[1].stiffness > result_linear.joints[1].stiffness);
}

#[test]
fn gravity_dir_is_root_for_all_joints() {
    let chain = make_chain(&[0, 1, 2]);
    let ramp = ChainRamp {
        root: SpringJointParam {
            gravity_dir: Vector3::new(1.0, 0.0, 0.0),
            ..Default::default()
        },
        tip: SpringJointParam {
            gravity_dir: Vector3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        shape: RampShape::Linear,
    };
    let result = apply_ramp(&chain, &ramp);

    for joint in &result.joints {
        assert_eq!(joint.gravity_dir, Vector3::new(1.0, 0.0, 0.0));
    }
}

#[test]
fn bone_ids_preserved_from_input_chain() {
    let chain = make_chain(&[5, 12, 7]);
    let ramp = ramp(0.8, 0.2, RampShape::Linear);
    let result = apply_ramp(&chain, &ramp);

    assert_eq!(result.joints[0].bone_id, 5);
    assert_eq!(result.joints[1].bone_id, 12);
    assert_eq!(result.joints[2].bone_id, 7);
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
        stiffness: 0.9,
        ..Default::default()
    };
    ramp.set_end(RampEnd::Root, joint);
    assert_eq!(ramp.root.bone_id, 5);
    assert_eq!(ramp.root.stiffness, 0.9);
}

#[test]
fn set_end_tip() {
    let mut ramp: ChainRamp = Default::default();
    let joint = SpringJointParam {
        bone_id: 7,
        stiffness: 0.1,
        ..Default::default()
    };
    ramp.set_end(RampEnd::Tip, joint);
    assert_eq!(ramp.tip.bone_id, 7);
    assert_eq!(ramp.tip.stiffness, 0.1);
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
                stiffness: 0.8,
                ..Default::default()
            },
            SpringJointParam {
                bone_id: 2,
                stiffness: 0.5,
                ..Default::default()
            },
            SpringJointParam {
                bone_id: 3,
                stiffness: 0.2,
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
    assert_eq!(ramp.root.stiffness, 0.8);
    assert_eq!(ramp.tip.bone_id, 3);
    assert_eq!(ramp.tip.stiffness, 0.2);
}

#[test]
fn load_ends_empty_chain_is_noop() {
    let mut ramp: ChainRamp = Default::default();
    ramp.root = SpringJointParam {
        bone_id: 99,
        stiffness: 0.5,
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
    assert_eq!(ramp.root.stiffness, 0.5);
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
            "root_stiffness",
            "root_drag_force",
            "root_gravity_power",
            "root_gravity_dir",
            "root_hit_radius",
            "tip_stiffness",
            "tip_drag_force",
            "tip_gravity_power",
            "tip_gravity_dir",
            "tip_hit_radius"
        ]
    );
}
