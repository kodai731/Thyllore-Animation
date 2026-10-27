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

#[test]
fn spring_edit_bridge_ramp_linear_interpolation() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 3).unwrap();
    let ramp = ramp_input("linear");

    let result = apply_ramp_to_chains(&spring, &bones, &["Chain_0".to_string()], &ramp).unwrap();

    assert_eq!(result.chains.len(), 1);
    let chain = &result.chains[0];
    assert_eq!(chain.joints.len(), 3);
    let stiffness_of = |joint: &SidecarSpringJoint| match joint.force {
        SidecarJointForce::Vrm { stiffness, .. } => stiffness,
        SidecarJointForce::RestRelative { .. } => panic!("ramp keeps the VRM model"),
    };
    assert!((stiffness_of(&chain.joints[0]) - 1.0).abs() < 1e-6);
    assert!((stiffness_of(&chain.joints[1]) - 0.5).abs() < 1e-6);
    assert!((stiffness_of(&chain.joints[2]) - 0.0).abs() < 1e-6);
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
fn spring_edit_bridge_chains_to_draw_selected_head() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 1).unwrap();

    let result = chains_to_draw(&spring, &bones, &[1], None, "selected").unwrap();

    assert_eq!(result.lines.len(), 2);
    assert!(result.radii.is_empty());
}

#[test]
fn spring_edit_bridge_chains_to_draw_selected_joint_with_active() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 3).unwrap();

    let result = chains_to_draw(&spring, &bones, &[2], Some(2), "selected").unwrap();

    assert_eq!(result.lines.len(), 1);
    assert_eq!(result.radii.len(), 1);
}

#[test]
fn spring_edit_bridge_chains_to_draw_mode_all() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 1).unwrap();

    let result = chains_to_draw(&spring, &bones, &[], None, "all").unwrap();

    assert_eq!(result.lines.len(), 2);
}

#[test]
fn spring_edit_bridge_chains_to_draw_unknown_mode_returns_error() {
    let bones = test_bones();
    let spring = add_chains_under_bone(&SidecarSpring::default(), &bones, "Head", 1).unwrap();

    let result = chains_to_draw(&spring, &bones, &[], None, "unknown");
    assert!(result.is_err());
}
