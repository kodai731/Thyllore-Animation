use super::*;
use crate::avatar_setup_bridge::BoneSpec;
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringJoint;

fn spec(name: &str, parent: Option<usize>) -> BoneSpec {
    BoneSpec {
        name: name.to_string(),
        parent,
        rest_position: [0.0, 0.0, 0.0],
        bind_matrix: None,
    }
}

fn test_specs() -> Vec<BoneSpec> {
    vec![
        spec("Hips", None),
        spec("Head", Some(0)),
        spec("hair_a", Some(1)),
        spec("hair_a.001", Some(2)),
        spec("hair_a.002", Some(3)),
        spec("hair_b", Some(1)),
        spec("hair_b.001", Some(5)),
    ]
}

fn bone(name: &str, parent: Option<usize>) -> BoneInput {
    BoneInput {
        name: name.to_string(),
        parent,
        rest_position: [0.0, 0.0, 0.0],
    }
}

fn ramp_input(shape: &str) -> RampInput {
    RampInput {
        root: JointValues {
            force: SidecarJointForce::Vrm {
                stiffness: 1.0,
                gravity_power: 0.0,
            },
            drag_force: 0.0,
            gravity_dir: [0.0, -1.0, 0.0],
            hit_radius: 0.0,
        },
        tip: JointValues {
            force: SidecarJointForce::Vrm {
                stiffness: 0.0,
                gravity_power: 0.0,
            },
            drag_force: 0.0,
            gravity_dir: [0.0, -1.0, 0.0],
            hit_radius: 0.0,
        },
        shape: shape.to_string(),
        rigid_fraction: 0.0,
    }
}

fn vrm_stiffness_of(joint: &SidecarSpringJoint) -> f32 {
    match joint.force {
        SidecarJointForce::Vrm { stiffness, .. } => stiffness,
        SidecarJointForce::RestRelative { .. } => panic!("the chain stays on the VRM model"),
    }
}

#[test]
fn spring_edit_bridge_builds_two_chains_under_head() {
    let request = UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 1,
    };

    let result = use_existing_bones_as_springs(&request).unwrap();

    assert_eq!(result.chains.len(), 2);

    let chain_a = result
        .chains
        .iter()
        .find(|c| c.joints.first().is_some_and(|j| j.bone == "hair_a"))
        .expect("chain starting with hair_a");
    assert_eq!(chain_a.joints.len(), 3);
    assert_eq!(chain_a.joints.first().unwrap().bone, "hair_a");
    assert_eq!(chain_a.joints[1].bone, "hair_a.001");
    assert_eq!(chain_a.joints[2].bone, "hair_a.002");

    let chain_b = result
        .chains
        .iter()
        .find(|c| c.joints.first().is_some_and(|j| j.bone == "hair_b"))
        .expect("chain starting with hair_b");
    assert_eq!(chain_b.joints.len(), 2);
    assert_eq!(chain_b.joints.first().unwrap().bone, "hair_b");
    assert_eq!(chain_b.joints[1].bone, "hair_b.001");
}

#[test]
fn spring_edit_bridge_idempotent_call_does_not_duplicate_chains() {
    let first_request = UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 1,
    };
    let first = use_existing_bones_as_springs(&first_request).unwrap();
    assert_eq!(first.chains.len(), 2);

    let first_names: Vec<_> = first.chains.iter().map(|c| c.name.clone()).collect();

    let second_request = UseExistingBonesAsSpringsRequest {
        spring: first,
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 1,
    };
    let second = use_existing_bones_as_springs(&second_request).unwrap();

    assert_eq!(second.chains.len(), 2);

    let second_names: Vec<_> = second.chains.iter().map(|c| c.name.clone()).collect();
    assert_eq!(first_names, second_names);
}

#[test]
fn spring_edit_bridge_unknown_root_or_short_runs_return_error() {
    let bad_root = UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "nonexistent".to_string(),
        min_joints: 1,
    };
    assert!(use_existing_bones_as_springs(&bad_root).is_err());

    let too_short = UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 4,
    };
    assert!(use_existing_bones_as_springs(&too_short).is_err());
}

fn with_vrm_joints(mut spring: SidecarSpring) -> SidecarSpring {
    for joint in spring.chains.iter_mut().flat_map(|chain| &mut chain.joints) {
        joint.force = SidecarJointForce::Vrm {
            stiffness: 1.0,
            gravity_power: 0.0,
        };
    }
    spring
}

#[test]
fn spring_edit_bridge_ramp_rigid_fraction_holds_the_root_joints() {
    let spring = with_vrm_joints(
        use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
            spring: SidecarSpring::default(),
            bones: test_specs(),
            root_bone: "Head".to_string(),
            min_joints: 3,
        })
        .unwrap(),
    );
    let mut ramp = ramp_input("linear");
    ramp.rigid_fraction = 0.5;

    let request = ApplyRampRequest {
        spring,
        bones: test_specs(),
        chain_names: vec!["Chain_0".to_string()],
        ramp,
    };
    let result = apply_ramp_to_chains(&request).unwrap();

    let chain = &result.chains[0];
    assert!((vrm_stiffness_of(&chain.joints[0]) - 1.0).abs() < 1e-6);
    assert!((vrm_stiffness_of(&chain.joints[1]) - 1.0).abs() < 1e-6);
    assert!((vrm_stiffness_of(&chain.joints[2]) - 0.0).abs() < 1e-6);
}

#[test]
fn spring_edit_bridge_ramp_linear_interpolation() {
    let spring = with_vrm_joints(
        use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
            spring: SidecarSpring::default(),
            bones: test_specs(),
            root_bone: "Head".to_string(),
            min_joints: 3,
        })
        .unwrap(),
    );
    let ramp = ramp_input("linear");

    let request = ApplyRampRequest {
        spring,
        bones: test_specs(),
        chain_names: vec!["Chain_0".to_string()],
        ramp,
    };
    let result = apply_ramp_to_chains(&request).unwrap();

    assert_eq!(result.chains.len(), 1);
    let chain = &result.chains[0];
    assert_eq!(chain.joints.len(), 3);
    assert!((vrm_stiffness_of(&chain.joints[0]) - 1.0).abs() < 1e-6);
    assert!((vrm_stiffness_of(&chain.joints[1]) - 0.5).abs() < 1e-6);
    assert!((vrm_stiffness_of(&chain.joints[2]) - 0.0).abs() < 1e-6);
}

#[test]
fn spring_edit_bridge_ramp_of_another_force_model_is_an_error() {
    let spring = with_vrm_joints(
        use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
            spring: SidecarSpring::default(),
            bones: test_specs(),
            root_bone: "Head".to_string(),
            min_joints: 3,
        })
        .unwrap(),
    );
    let mut rest_relative_ramp = ramp_input("linear");
    for end in [&mut rest_relative_ramp.root, &mut rest_relative_ramp.tip] {
        end.force = SidecarJointForce::RestRelative {
            bending_relaxation: 0.5,
            gravity_scale: 1.0,
        };
    }

    let request = ApplyRampRequest {
        spring,
        bones: test_specs(),
        chain_names: vec!["Chain_0".to_string()],
        ramp: rest_relative_ramp,
    };
    let result = apply_ramp_to_chains(&request);

    assert!(result.is_err());
}

#[test]
fn spring_edit_bridge_chains_under_bone_head() {
    let spring = use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 1,
    })
    .unwrap();

    let request = ChainsUnderBoneRequest {
        spring,
        bones: test_specs(),
        bone_name: "Head".to_string(),
    };
    let result = chains_under_bone(&request).unwrap();

    assert_eq!(result.len(), 2);
    assert!(result.contains(&"Chain_0".to_string()));
    assert!(result.contains(&"Chain_1".to_string()));
}

#[test]
fn spring_edit_bridge_chains_in_scope_sibling_chains() {
    let spring = use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 1,
    })
    .unwrap();

    let request = ChainsInScopeRequest {
        spring,
        bones: test_specs(),
        chain_name: "Chain_0".to_string(),
        scope: RampScope::SiblingChains,
    };
    let result = chains_in_scope(&request).unwrap();

    assert_eq!(result.len(), 2);
}

#[test]
fn spring_edit_bridge_unknown_shape_returns_error() {
    let spring = use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 3,
    })
    .unwrap();
    let mut ramp = ramp_input("linear");
    ramp.shape = "unknown".to_string();

    let request = ApplyRampRequest {
        spring,
        bones: test_specs(),
        chain_names: vec!["Chain_0".to_string()],
        ramp,
    };
    let result = apply_ramp_to_chains(&request);
    assert!(result.is_err());
}

#[test]
fn spring_edit_bridge_chains_with_radius_follow_the_active_joint() {
    let spring = use_existing_bones_as_springs(&UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones: test_specs(),
        root_bone: "Head".to_string(),
        min_joints: 3,
    })
    .unwrap();

    let request_none = ChainsWithRadiusRequest {
        spring: spring.clone(),
        bones: test_specs(),
        active: None,
        mode: SpringChainGizmoMode::All,
    };
    assert!(chains_with_radius(&request_none).unwrap().is_empty());

    let request_some = ChainsWithRadiusRequest {
        spring: spring.clone(),
        bones: test_specs(),
        active: Some(2),
        mode: SpringChainGizmoMode::All,
    };
    assert_eq!(chains_with_radius(&request_some).unwrap().len(), 1);

    let request_mode_none = ChainsWithRadiusRequest {
        spring,
        bones: test_specs(),
        active: Some(2),
        mode: SpringChainGizmoMode::None,
    };
    assert!(chains_with_radius(&request_mode_none).unwrap().is_empty());
}

#[test]
fn spring_edit_bridge_resolves_humanoid_role_on_prefixed_bones() {
    let bones: Vec<BoneSpec> = vec![
        spec("mixamorig:Hips", None),
        spec("mixamorig:Spine", Some(0)),
        spec("mixamorig:Neck", Some(1)),
        spec("mixamorig:Head", Some(2)),
        spec("hair_a", Some(3)),
        spec("hair_a.001", Some(4)),
        spec("hair_b", Some(3)),
        spec("hair_b.001", Some(6)),
        spec("hair_b.002", Some(7)),
    ];

    let request = UseExistingBonesAsSpringsRequest {
        spring: SidecarSpring::default(),
        bones,
        root_bone: "Head".to_string(),
        min_joints: 1,
    };
    let result = use_existing_bones_as_springs(&request).unwrap();

    assert_eq!(result.chains.len(), 2);

    let chain_a = result
        .chains
        .iter()
        .find(|c| c.joints.first().is_some_and(|j| j.bone == "hair_a"))
        .expect("chain starting with hair_a");
    assert_eq!(chain_a.joints.len(), 2);
    assert_eq!(chain_a.joints.first().unwrap().bone, "hair_a");
    assert_eq!(chain_a.joints[1].bone, "hair_a.001");

    let chain_b = result
        .chains
        .iter()
        .find(|c| c.joints.first().is_some_and(|j| j.bone == "hair_b"))
        .expect("chain starting with hair_b");
    assert_eq!(chain_b.joints.len(), 3);
    assert_eq!(chain_b.joints.first().unwrap().bone, "hair_b");
    assert_eq!(chain_b.joints[1].bone, "hair_b.001");
    assert_eq!(chain_b.joints[2].bone, "hair_b.002");
}

#[test]
fn spring_edit_bridge_resolve_root_bone_name_maps_role_to_prefixed_bone() {
    let bones: Vec<BoneSpec> = vec![
        spec("mixamorig:Hips", None),
        spec("mixamorig:Spine", Some(0)),
        spec("mixamorig:Neck", Some(1)),
        spec("mixamorig:Head", Some(2)),
    ];
    let request = |root_bone: &str| ResolveRootBoneRequest {
        bones: bones.clone(),
        root_bone: root_bone.to_string(),
    };

    let resolved = resolve_root_bone_name(&request("Head")).unwrap();
    assert_eq!(resolved, "mixamorig:Head");

    let resolved = resolve_root_bone_name(&request("mixamorig:Hips")).unwrap();
    assert_eq!(resolved, "mixamorig:Hips");

    let err = resolve_root_bone_name(&request("Unknown")).unwrap_err();
    assert!(err.to_string().contains("not found"));
}
