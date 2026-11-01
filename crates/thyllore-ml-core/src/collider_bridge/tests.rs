use super::*;
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarJointForce;

fn make_bones() -> Vec<FitBone> {
    let identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    vec![
        FitBone {
            bone: BoneSpec {
                name: "Hips".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
                rest_frame: None,
            },
            bind_matrix: identity,
        },
        FitBone {
            bone: BoneSpec {
                name: "Spine".into(),
                parent: Some(0),
                rest_position: [0.0, 1.1, 0.0],
                rest_frame: None,
            },
            bind_matrix: identity,
        },
        FitBone {
            bone: BoneSpec {
                name: "Chest".into(),
                parent: Some(1),
                rest_position: [0.0, 1.2, 0.0],
                rest_frame: None,
            },
            bind_matrix: identity,
        },
        FitBone {
            bone: BoneSpec {
                name: "UpperChest".into(),
                parent: Some(2),
                rest_position: [0.0, 1.25, 0.0],
                rest_frame: None,
            },
            bind_matrix: identity,
        },
        FitBone {
            bone: BoneSpec {
                name: "Neck".into(),
                parent: Some(3),
                rest_position: [0.0, 1.3, 0.0],
                rest_frame: None,
            },
            bind_matrix: identity,
        },
        FitBone {
            bone: BoneSpec {
                name: "Head".into(),
                parent: Some(4),
                rest_position: [0.0, 1.4, 0.0],
                rest_frame: None,
            },
            bind_matrix: identity,
        },
    ]
}

fn make_cylinder_skin(bones: &[FitBone]) -> Vec<FitSkin> {
    let spine_idx = bones.iter().position(|b| b.bone.name == "Spine").unwrap();
    let mut positions = Vec::new();
    let mut influences = Vec::new();
    let radius = 0.1;
    let levels = 5;
    for level in 0..levels {
        let y = -0.2 + (level as f32 / (levels - 1) as f32) * 0.4;
        let circles_per_level = 4;
        for c in 0..circles_per_level {
            let angle = 2.0 * std::f32::consts::PI * (c as f32 / circles_per_level as f32);
            let x = radius * angle.cos();
            let z = radius * angle.sin();
            positions.push([x, y, z]);
            influences.push(vec![(spine_idx, 1.0)]);
        }
    }
    vec![FitSkin {
        positions,
        influences,
    }]
}

fn empty_spring() -> SidecarSpring {
    SidecarSpring::default()
}

#[test]
fn collider_bridge_fit_produces_collider_on_spine() {
    let bones = make_bones();
    let skins = make_cylinder_skin(&bones);
    let spring = empty_spring();

    let result = fit_body_colliders(&spring, &bones, &skins, &[]).unwrap();

    assert!(
        !result.colliders.is_empty(),
        "fit should produce at least one collider"
    );
}

#[test]
fn collider_bridge_fit_has_fitted_body_group_on_all_chains() {
    let mut spring = empty_spring();
    spring.chains.push(
        thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringChain {
            name: "Spine".into(),
            center: None,
            collider_groups: Vec::new(),
            joints: vec![
                thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringJoint {
                    bone: "Spine".into(),
                    force: SidecarJointForce::Vrm {
                        stiffness: 0.5,
                        gravity_power: 1.0,
                    },
                    drag_force: 0.0,
                    gravity_dir: [0.0, -1.0, 0.0],
                    hit_radius: 0.0,
                },
            ],
            tail: None,
            enabled: true,
            origin: Default::default(),
        },
    );

    let bones = make_bones();
    let skins = make_cylinder_skin(&bones);

    let result = fit_body_colliders(&spring, &bones, &skins, &[]).unwrap();

    let fitted_group = result
        .collider_groups
        .iter()
        .find(|g| g.name == "Fitted Body");
    assert!(
        fitted_group.is_some(),
        "should have a 'Fitted Body' collider group"
    );

    for chain in &result.chains {
        assert!(
            chain.collider_groups.contains(&"Fitted Body".to_string()),
            "chain '{}' should reference 'Fitted Body' group",
            chain.name
        );
    }
}

#[test]
fn collider_bridge_assign_collider_groups_replaces() {
    let mut spring = empty_spring();
    spring.chains.push(
        thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringChain {
            name: "Spine".into(),
            center: None,
            collider_groups: vec!["Existing".to_string()],
            joints: vec![
                thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringJoint {
                    bone: "Spine".into(),
                    force: SidecarJointForce::Vrm {
                        stiffness: 0.5,
                        gravity_power: 1.0,
                    },
                    drag_force: 0.0,
                    gravity_dir: [0.0, -1.0, 0.0],
                    hit_radius: 0.0,
                },
            ],
            tail: None,
            enabled: true,
            origin: Default::default(),
        },
    );
    spring.collider_groups.push(
        thyllore_avatar_core::vrchat::sidecar_spring::SidecarColliderGroup {
            name: "Existing".to_string(),
            colliders: Vec::new(),
        },
    );
    spring.collider_groups.push(
        thyllore_avatar_core::vrchat::sidecar_spring::SidecarColliderGroup {
            name: "New".to_string(),
            colliders: Vec::new(),
        },
    );

    let bones = make_bones();
    let bone_inputs = to_bone_inputs(&bones);

    let result = assign_collider_groups(
        &spring,
        &bone_inputs,
        &["Spine".to_string()],
        &["New".to_string()],
    )
    .unwrap();

    let chain = &result.chains[0];
    assert_eq!(chain.collider_groups, vec!["New".to_string()]);
}

#[test]
fn collider_bridge_assign_collider_groups_unknown_group_is_err() {
    let mut spring = empty_spring();
    spring.chains.push(
        thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringChain {
            name: "Spine".into(),
            center: None,
            collider_groups: Vec::new(),
            joints: vec![
                thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpringJoint {
                    bone: "Spine".into(),
                    force: SidecarJointForce::Vrm {
                        stiffness: 0.5,
                        gravity_power: 1.0,
                    },
                    drag_force: 0.0,
                    gravity_dir: [0.0, -1.0, 0.0],
                    hit_radius: 0.0,
                },
            ],
            tail: None,
            enabled: true,
            origin: Default::default(),
        },
    );
    let bones = make_bones();
    let bone_inputs = to_bone_inputs(&bones);

    let err = assign_collider_groups(
        &spring,
        &bone_inputs,
        &["Spine".to_string()],
        &["Unknown".to_string()],
    )
    .unwrap_err();

    assert!(
        err.to_string().contains("Unknown"),
        "error should mention 'Unknown': {}",
        err
    );
}

#[test]
fn collider_bridge_fit_high_bone_index_head() {
    let bones = make_bones();
    let head_idx = bones.iter().position(|b| b.bone.name == "Head").unwrap();
    assert!(head_idx >= 5);

    let mut positions = Vec::new();
    let mut influences = Vec::new();
    let radius = 0.1;
    let levels = 5;
    for level in 0..levels {
        let y = -0.2 + (level as f32 / (levels - 1) as f32) * 0.4;
        for c in 0..4 {
            let angle = 2.0 * std::f32::consts::PI * (c as f32 / 4.0);
            let x = radius * angle.cos();
            let z = radius * angle.sin();
            positions.push([x, y, z]);
            influences.push(vec![(head_idx, 1.0)]);
        }
    }
    let skins = vec![FitSkin {
        positions,
        influences,
    }];

    let spring = empty_spring();
    let result = fit_body_colliders(&spring, &bones, &skins, &[]).unwrap();

    assert!(
        result
            .colliders
            .iter()
            .any(|collider| collider.bone == "Head"),
        "fit should produce a collider on Head (bone index {})",
        head_idx
    );
}
