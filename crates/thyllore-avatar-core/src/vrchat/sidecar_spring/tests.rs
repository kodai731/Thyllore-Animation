use cgmath::Vector3;
use serde_json::json;
use thyllore_anim_core::spring_bone::{
    ChainOrigin, ChainRole, ColliderShape, SpringBoneSetup, SpringChain, SpringColliderDef,
    SpringColliderGroup, SpringCollisionMode, SpringJointParam, SubmeshSelector,
    FITTED_BODY_GROUP_NAME,
};
use thyllore_anim_core::BoneId;

use super::*;
use crate::humanoid::components::skeleton_input::BoneInput;

fn make_bones(names: &[&str]) -> Vec<BoneInput> {
    names
        .iter()
        .enumerate()
        .map(|(i, name)| BoneInput {
            name: name.to_string(),
            parent: if i == 0 { None } else { Some(i - 1) },
            rest_position: [0.0, 0.0, 0.0],
        })
        .collect()
}

#[test]
fn test_full_mapping() {
    let bones = make_bones(&["Hips", "Spine", "Tail1", "Tail2"]);

    let mut setup = SpringBoneSetup::default();
    setup.collision_mode = SpringCollisionMode::VrmCompatible;

    setup.colliders.push(SpringColliderDef {
        id: 0,
        bone_id: 0,
        offset: Vector3::new(0.0, 0.1, 0.0),
        shape: ColliderShape::Sphere { radius: 0.05 },
    });

    setup.colliders.push(SpringColliderDef {
        id: 1,
        bone_id: 1,
        offset: Vector3::new(0.0, -0.2, 0.0),
        shape: ColliderShape::Capsule {
            radius: 0.03,
            tail: Vector3::new(0.0, 0.1, 0.0),
        },
    });

    setup.collider_groups.push(SpringColliderGroup {
        id: 0,
        name: "Body".to_string(),
        collider_ids: vec![0, 1],
    });

    setup.chains.push(SpringChain {
        id: 0,
        name: "Tail".to_string(),
        joints: vec![
            SpringJointParam {
                bone_id: 2,
                stiffness: 0.8,
                drag_force: 0.5,
                gravity_power: 1.0,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.02,
            },
            SpringJointParam {
                bone_id: 3,
                stiffness: 0.6,
                drag_force: 0.4,
                gravity_power: 0.5,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.01,
            },
        ],
        collider_group_ids: vec![0],
        center_bone_id: Some(0),
        tail_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });

    setup.next_chain_id = 1;
    setup.next_collider_id = 2;
    setup.next_group_id = 1;

    let sidecar = build_sidecar_spring(&setup, &bones);

    assert_eq!(sidecar.collision_mode, SidecarCollisionMode::VrmCompatible);

    assert_eq!(sidecar.colliders.len(), 2);
    assert_eq!(sidecar.colliders[0].name, "Hips#0");
    assert_eq!(sidecar.colliders[0].bone, "Hips");
    assert_eq!(sidecar.colliders[0].offset, [0.0, 0.1, 0.0]);
    assert_eq!(sidecar.colliders[0].shape, SidecarColliderShape::Sphere);
    assert_eq!(sidecar.colliders[0].radius, 0.05);
    assert_eq!(sidecar.colliders[0].tail, None);
    assert_eq!(sidecar.colliders[0].origin, SidecarColliderOrigin::Authored);

    assert_eq!(sidecar.colliders[1].name, "Spine#1");
    assert_eq!(sidecar.colliders[1].bone, "Spine");
    assert_eq!(sidecar.colliders[1].offset, [0.0, -0.2, 0.0]);
    assert_eq!(sidecar.colliders[1].shape, SidecarColliderShape::Capsule);
    assert_eq!(sidecar.colliders[1].radius, 0.03);
    assert_eq!(sidecar.colliders[1].tail, Some([0.0, 0.1, 0.0]));
    assert_eq!(sidecar.colliders[1].origin, SidecarColliderOrigin::Authored);

    assert_eq!(sidecar.collider_groups.len(), 1);
    assert_eq!(sidecar.collider_groups[0].name, "Body");
    assert_eq!(
        sidecar.collider_groups[0].colliders,
        vec!["Hips#0", "Spine#1"]
    );

    assert_eq!(sidecar.chains.len(), 1);
    let chain = &sidecar.chains[0];
    assert_eq!(chain.name, "Tail");
    assert_eq!(chain.center, Some("Hips".to_string()));
    assert_eq!(chain.collider_groups, vec!["Body"]);
    assert_eq!(chain.joints.len(), 2);

    let j0 = &chain.joints[0];
    assert_eq!(j0.bone, "Tail1");
    assert_eq!(j0.stiffness, 0.8);
    assert_eq!(j0.drag_force, 0.5);
    assert_eq!(j0.gravity_power, 1.0);
    assert_eq!(j0.gravity_dir, [0.0, -1.0, 0.0]);
    assert_eq!(j0.hit_radius, 0.02);

    let j1 = &chain.joints[1];
    assert_eq!(j1.bone, "Tail2");
    assert_eq!(j1.stiffness, 0.6);
    assert_eq!(j1.drag_force, 0.4);
    assert_eq!(j1.gravity_power, 0.5);
    assert_eq!(j1.gravity_dir, [0.0, -1.0, 0.0]);
    assert_eq!(j1.hit_radius, 0.01);
}

#[test]
fn test_fitted_body_origin() {
    let bones = make_bones(&["Hips", "Spine"]);

    let mut setup = SpringBoneSetup::default();

    setup.colliders.push(SpringColliderDef {
        id: 0,
        bone_id: 0,
        offset: Vector3::new(0.0, 0.0, 0.0),
        shape: ColliderShape::Sphere { radius: 0.1 },
    });

    setup.collider_groups.push(SpringColliderGroup {
        id: 0,
        name: FITTED_BODY_GROUP_NAME.to_string(),
        collider_ids: vec![0],
    });

    let sidecar = build_sidecar_spring(&setup, &bones);

    assert_eq!(sidecar.colliders.len(), 1);
    let c = &sidecar.colliders[0];
    assert_eq!(c.origin, SidecarColliderOrigin::Fitted);
    assert_eq!(c.name, format!("{FITTED_BODY_GROUP_NAME}/Hips"));
}

#[test]
fn setup_round_trips_through_the_sidecar() {
    let bones = make_bones(&["Hips", "Spine", "Tail1", "Tail2", "Tail3"]);
    let mut setup = SpringBoneSetup::default();
    setup.collision_mode = SpringCollisionMode::SegmentExtended;
    setup.colliders.push(SpringColliderDef {
        id: 4,
        bone_id: 0,
        offset: Vector3::new(0.0, 0.1, 0.0),
        shape: ColliderShape::Capsule {
            radius: 0.05,
            tail: Vector3::new(0.0, 0.2, 0.0),
        },
    });
    setup.collider_groups.push(SpringColliderGroup {
        id: 7,
        name: "Body".to_string(),
        collider_ids: vec![4],
    });
    setup.chains.push(SpringChain {
        id: 2,
        name: "tail".to_string(),
        joints: vec![
            SpringJointParam {
                bone_id: 2,
                stiffness: 0.3,
                hit_radius: 0.02,
                ..Default::default()
            },
            SpringJointParam {
                bone_id: 3,
                stiffness: 0.6,
                hit_radius: 0.03,
                ..Default::default()
            },
        ],
        collider_group_ids: vec![7],
        center_bone_id: Some(1),
        tail_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });
    setup.next_chain_id = 3;
    setup.next_collider_id = 5;
    setup.next_group_id = 8;

    let sidecar = build_sidecar_spring(&setup, &bones);
    assert_eq!(sidecar.chains[0].joints.len(), 3, "tail joint appended");
    assert_eq!(sidecar.chains[0].tail.as_deref(), Some("Tail3"));

    let restored = spring_setup_from_sidecar(&sidecar, &bones);

    assert_eq!(
        restored.collision_mode,
        SpringCollisionMode::SegmentExtended
    );
    assert_eq!(restored.chains.len(), 1);
    let chain = &restored.chains[0];
    assert_eq!(chain.name, "tail");
    assert_eq!(chain.center_bone_id, Some(1));
    assert_eq!(chain.tail_bone_id, Some(4));
    let joint_bones: Vec<_> = chain.joints.iter().map(|j| j.bone_id).collect();
    assert_eq!(joint_bones, vec![2, 3]);
    assert_eq!(chain.joints[1].stiffness, 0.6);
    assert_eq!(chain.joints[1].hit_radius, 0.03);
    assert_eq!(
        chain.collider_group_ids,
        vec![restored.collider_groups[0].id]
    );
    assert_eq!(
        restored.collider_groups[0].collider_ids,
        vec![restored.colliders[0].id]
    );
    assert_eq!(restored.colliders[0].bone_id, 0);
    match restored.colliders[0].shape {
        ColliderShape::Capsule { radius, tail } => {
            assert_eq!(radius, 0.05);
            assert_eq!(tail, Vector3::new(0.0, 0.2, 0.0));
        }
        ColliderShape::Sphere { .. } => panic!("capsule lost"),
    }
    assert_eq!(restored.next_chain_id, 1);
    assert_eq!(restored.next_collider_id, 1);
    assert_eq!(restored.next_group_id, 1);
}

#[test]
fn generated_origin_round_trips_and_is_absent_for_authored_chains() {
    let bones = make_bones(&["Head", "Hair0", "Hair1", "Side0"]);
    let origin = ChainOrigin::Generated {
        target: SubmeshSelector::from_parts("Hair_back", Some("Hair")),
        role: ChainRole::Hair,
    };
    let joints = |ids: &[BoneId]| -> Vec<SpringJointParam> {
        ids.iter()
            .map(|&bone_id| SpringJointParam {
                bone_id,
                ..Default::default()
            })
            .collect()
    };
    let setup = SpringBoneSetup {
        chains: vec![
            SpringChain {
                id: 0,
                joints: joints(&[1, 2]),
                origin: origin.clone(),
                ..Default::default()
            },
            SpringChain {
                id: 1,
                joints: joints(&[3]),
                ..Default::default()
            },
        ],
        next_chain_id: 2,
        ..Default::default()
    };

    let sidecar = build_sidecar_spring(&setup, &bones);
    let json = serde_json::to_string(&sidecar).unwrap();
    assert!(json
        .contains("\"origin\":{\"mesh\":\"Hair_back\",\"material\":\"Hair\",\"role\":\"hair\"}"));
    assert_eq!(sidecar.chains[1].origin, None);

    let restored = spring_setup_from_sidecar(&serde_json::from_str(&json).unwrap(), &bones);
    assert_eq!(restored.chains[0].origin, origin);
    assert_eq!(restored.chains[1].origin, ChainOrigin::Authored);
}

#[test]
fn chain_without_origin_field_loads_as_authored() {
    let bones = make_bones(&["Head", "Hair0"]);
    let json = r#"{"collision_mode":"vrm_compatible","colliders":[],"collider_groups":[],
        "chains":[{"name":"c","center":null,"collider_groups":[],
        "joints":[{"bone":"Hair0","stiffness":1.0,"drag_force":0.4,"gravity_power":0.0,"gravity_dir":[0.0,-1.0,0.0],"hit_radius":0.02}]}]}"#;
    let sidecar: SidecarSpring = serde_json::from_str(json).unwrap();
    let restored = spring_setup_from_sidecar(&sidecar, &bones);
    assert_eq!(restored.chains[0].origin, ChainOrigin::Authored);
}

#[test]
fn chain_ending_at_a_leaf_keeps_every_joint_on_import() {
    let bones = make_bones(&["Hips", "Hair0", "Hair1"]);
    let mut setup = SpringBoneSetup::default();
    setup.chains.push(SpringChain {
        joints: vec![
            SpringJointParam {
                bone_id: 1,
                ..Default::default()
            },
            SpringJointParam {
                bone_id: 2,
                ..Default::default()
            },
        ],
        ..Default::default()
    });

    let sidecar = build_sidecar_spring(&setup, &bones);
    assert_eq!(sidecar.chains[0].tail, None);

    let restored = spring_setup_from_sidecar(&sidecar, &bones);
    let joint_bones: Vec<_> = restored.chains[0]
        .joints
        .iter()
        .map(|j| j.bone_id)
        .collect();
    assert_eq!(joint_bones, vec![1, 2]);
}

#[test]
fn test_serde_format() {
    let bones = make_bones(&["Hips", "Spine"]);

    let mut setup = SpringBoneSetup::default();
    setup.collision_mode = SpringCollisionMode::VrmCompatible;

    setup.colliders.push(SpringColliderDef {
        id: 0,
        bone_id: 0,
        offset: Vector3::new(0.0, 0.0, 0.0),
        shape: ColliderShape::Capsule {
            radius: 0.05,
            tail: Vector3::new(0.0, 0.1, 0.0),
        },
    });

    let sidecar = build_sidecar_spring(&setup, &bones);
    let value = serde_json::to_value(&sidecar).unwrap();

    assert_eq!(value["collision_mode"], json!("vrm_compatible"));
    assert_eq!(value["colliders"][0]["shape"], json!("capsule"));
}

#[test]
fn test_out_of_range_bone_id_filtered() {
    let bones = make_bones(&["Hips", "Spine"]);

    let mut setup = SpringBoneSetup::default();

    setup.colliders.push(SpringColliderDef {
        id: 0,
        bone_id: 99,
        offset: Vector3::new(0.0, 0.0, 0.0),
        shape: ColliderShape::Sphere { radius: 0.1 },
    });

    setup.colliders.push(SpringColliderDef {
        id: 1,
        bone_id: 1,
        offset: Vector3::new(0.0, 0.0, 0.0),
        shape: ColliderShape::Sphere { radius: 0.1 },
    });

    setup.collider_groups.push(SpringColliderGroup {
        id: 0,
        name: "Test".to_string(),
        collider_ids: vec![0, 1],
    });

    let sidecar = build_sidecar_spring(&setup, &bones);

    assert_eq!(sidecar.colliders.len(), 1);
    assert_eq!(sidecar.colliders[0].name, "Spine#1");

    assert_eq!(sidecar.collider_groups.len(), 1);
    assert_eq!(sidecar.collider_groups[0].colliders, vec!["Spine#1"]);
}

#[test]
fn test_tail_joint_appended_when_child_exists() {
    let bones = make_bones(&["Hips", "Spine", "Tail1", "Tail2", "TailTip"]);

    let mut setup = SpringBoneSetup::default();

    setup.chains.push(SpringChain {
        id: 0,
        name: "Tail".to_string(),
        joints: vec![
            SpringJointParam {
                bone_id: 1,
                stiffness: 0.8,
                drag_force: 0.5,
                gravity_power: 1.0,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.02,
            },
            SpringJointParam {
                bone_id: 2,
                stiffness: 0.6,
                drag_force: 0.4,
                gravity_power: 0.5,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.01,
            },
        ],
        collider_group_ids: vec![],
        center_bone_id: None,
        tail_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });

    let sidecar = build_sidecar_spring(&setup, &bones);

    assert_eq!(sidecar.chains.len(), 1);
    let chain = &sidecar.chains[0];
    assert_eq!(chain.joints.len(), 3);
    assert_eq!(chain.joints[0].bone, "Spine");
    assert_eq!(chain.joints[1].bone, "Tail1");
    assert_eq!(chain.joints[2].bone, "Tail2");
}

#[test]
fn test_no_tail_joint_when_no_child() {
    let bones = make_bones(&["Hips", "Spine", "Tail1"]);

    let mut setup = SpringBoneSetup::default();

    setup.chains.push(SpringChain {
        id: 0,
        name: "Tail".to_string(),
        joints: vec![
            SpringJointParam {
                bone_id: 1,
                stiffness: 0.8,
                drag_force: 0.5,
                gravity_power: 1.0,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.02,
            },
            SpringJointParam {
                bone_id: 2,
                stiffness: 0.6,
                drag_force: 0.4,
                gravity_power: 0.5,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.01,
            },
        ],
        collider_group_ids: vec![],
        center_bone_id: None,
        tail_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });

    let sidecar = build_sidecar_spring(&setup, &bones);

    assert_eq!(sidecar.chains.len(), 1);
    let chain = &sidecar.chains[0];
    assert_eq!(chain.joints.len(), 2);
    assert_eq!(chain.joints[0].bone, "Spine");
    assert_eq!(chain.joints[1].bone, "Tail1");
}
