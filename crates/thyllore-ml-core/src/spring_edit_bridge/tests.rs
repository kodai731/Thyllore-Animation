use super::*;
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringJoint;

fn bone(name: &str, parent: Option<usize>) -> BoneInput {
    BoneInput {
        name: name.to_string(),
        parent,
        rest_position: [0.0, 0.0, 0.0],
    }
}

fn test_bones() -> Vec<BoneInput> {
    vec![
        bone("Hips", None),
        bone("Head", Some(0)),
        bone("hair_a", Some(1)),
        bone("hair_a.001", Some(2)),
        bone("hair_a.002", Some(3)),
        bone("hair_b", Some(1)),
        bone("hair_b.001", Some(5)),
    ]
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
    }
}

#[test]
fn spring_edit_bridge_builds_two_chains_under_head() {
    let bones = test_bones();
    let spring = SidecarSpring::default();

    let result = add_chains_under_bone(&spring, &bones, "Head", 1).unwrap();

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
    let bones = test_bones();
    let spring = SidecarSpring::default();

    let first = add_chains_under_bone(&spring, &bones, "Head", 1).unwrap();
    assert_eq!(first.chains.len(), 2);

    let second = add_chains_under_bone(&first, &bones, "Head", 1).unwrap();

    assert_eq!(second.chains.len(), 2);

    let first_names: Vec<_> = first.chains.iter().map(|c| c.name.clone()).collect();
    let second_names: Vec<_> = second.chains.iter().map(|c| c.name.clone()).collect();
    assert_eq!(first_names, second_names);
}

#[test]
fn spring_edit_bridge_unknown_root_or_short_runs_return_error() {
    let bones = test_bones();
    let spring = SidecarSpring::default();

    assert!(add_chains_under_bone(&spring, &bones, "nonexistent", 1).is_err());
    assert!(add_chains_under_bone(&spring, &bones, "Head", 4).is_err());
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
fn spring_edit_bridge_ramp_linear_interpolation() {
    let bones = test_bones();
    let spring = with_vrm_joints(
        add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 3).unwrap(),
    );
    let ramp = ramp_input("linear");

    let result = apply_ramp_to_chains(&spring, &bones, &["Chain_0".to_string()], &ramp).unwrap();

    assert_eq!(result.chains.len(), 1);
    let chain = &result.chains[0];
    assert_eq!(chain.joints.len(), 3);
    let stiffness_of = |joint: &SidecarSpringJoint| match joint.force {
        SidecarJointForce::Vrm { stiffness, .. } => stiffness,
        SidecarJointForce::RestRelative { .. } => panic!("the chain stays on the VRM model"),
    };
    assert!((stiffness_of(&chain.joints[0]) - 1.0).abs() < 1e-6);
    assert!((stiffness_of(&chain.joints[1]) - 0.5).abs() < 1e-6);
    assert!((stiffness_of(&chain.joints[2]) - 0.0).abs() < 1e-6);
}

#[test]
fn spring_edit_bridge_ramp_of_another_force_model_is_an_error() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 3).unwrap();

    let result = apply_ramp_to_chains(
        &spring,
        &bones,
        &["Chain_0".to_string()],
        &ramp_input("linear"),
    );

    assert!(result.is_err());
}

#[test]
fn spring_edit_bridge_chains_under_bone_head() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 1).unwrap();

    let result = chains_under_bone(&spring, &bones, "Head").unwrap();

    assert_eq!(result.len(), 2);
    assert!(result.contains(&"Chain_0".to_string()));
    assert!(result.contains(&"Chain_1".to_string()));
}

#[test]
fn spring_edit_bridge_chains_in_scope_sibling_chains() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 1).unwrap();

    let result = chains_in_scope(&spring, &bones, "Chain_0", "sibling_chains").unwrap();

    assert_eq!(result.len(), 2);
}

#[test]
fn spring_edit_bridge_unknown_shape_returns_error() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 3).unwrap();
    let mut ramp = ramp_input("linear");
    ramp.shape = "unknown".to_string();

    let result = apply_ramp_to_chains(&spring, &bones, &["Chain_0".to_string()], &ramp);
    assert!(result.is_err());
}

#[test]
fn spring_edit_bridge_chains_with_radius_follow_the_active_joint() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 3).unwrap();

    assert!(chains_with_radius(&spring, &bones, None, "all")
        .unwrap()
        .is_empty());
    assert_eq!(
        chains_with_radius(&spring, &bones, Some(2), "all")
            .unwrap()
            .len(),
        1
    );
    assert!(chains_with_radius(&spring, &bones, Some(2), "none")
        .unwrap()
        .is_empty());
}

#[test]
fn spring_edit_bridge_chains_with_radius_unknown_mode_returns_error() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 1).unwrap();

    assert!(chains_with_radius(&spring, &bones, None, "unknown").is_err());
}

#[test]
fn spring_edit_bridge_resolves_humanoid_role_on_prefixed_bones() {
    let bones: Vec<BoneInput> = vec![
        bone("mixamorig:Hips", None),
        bone("mixamorig:Spine", Some(0)),
        bone("mixamorig:Neck", Some(1)),
        bone("mixamorig:Head", Some(2)),
        bone("hair_a", Some(3)),
        bone("hair_a.001", Some(4)),
        bone("hair_b", Some(3)),
        bone("hair_b.001", Some(6)),
        bone("hair_b.002", Some(7)),
    ];
    let spring = SidecarSpring::default();

    let result = add_chains_under_bone(&spring, &bones, "Head", 1).unwrap();

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
    let bones: Vec<BoneInput> = vec![
        bone("mixamorig:Hips", None),
        bone("mixamorig:Spine", Some(0)),
        bone("mixamorig:Neck", Some(1)),
        bone("mixamorig:Head", Some(2)),
    ];

    let resolved = resolve_root_bone_name(&bones, "Head").unwrap();
    assert_eq!(resolved, "mixamorig:Head");

    let resolved = resolve_root_bone_name(&bones, "mixamorig:Hips").unwrap();
    assert_eq!(resolved, "mixamorig:Hips");

    let err = resolve_root_bone_name(&bones, "Unknown").unwrap_err();
    assert!(err.to_string().contains("not found"));
}
