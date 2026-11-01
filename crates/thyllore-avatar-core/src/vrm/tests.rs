use std::collections::BTreeMap;

use cgmath::{Matrix3, SquareMatrix};
use thyllore_anim_core::spring_bone::{ColliderShape, JointForce, VrmJointForce};
use thyllore_importer_core::gltf::spring_bone_extension::parse_vrmc_spring_bone;

use super::*;
use crate::humanoid::components::skeleton_input::BoneInput;
use crate::stats::components::stats::AvatarStats;
use crate::vrchat::sidecar::{AvatarSidecar, SidecarBlink, SIDECAR_SCHEMA_VERSION};
use crate::vrchat::sidecar_spring::{
    BoneRestFrame, SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin,
    SidecarColliderShape, SidecarCollisionMode, SidecarJointForce, SidecarSpring,
    SidecarSpringChain, SidecarSpringJoint,
};

const BONE_NAMES: [&str; 5] = ["Hips", "Head", "Hair1", "Hair2", "Hair3"];
const METRES_PER_BONE_UNIT: f32 = 0.01;

fn bones() -> Vec<BoneInput> {
    let parents = [None, Some(0), Some(1), Some(2), Some(3)];
    let heights = [1.0, 1.5, 1.6, 1.5, 1.4];
    BONE_NAMES
        .iter()
        .zip(parents)
        .zip(heights)
        .map(|((name, parent), height)| BoneInput {
            name: name.to_string(),
            parent,
            rest_position: [0.0, height, 0.0],
        })
        .collect()
}

fn centimetre_frames() -> Vec<BoneRestFrame> {
    let frame = BoneRestFrame::new(Matrix3::from_value(METRES_PER_BONE_UNIT)).expect("regular");
    vec![frame; BONE_NAMES.len()]
}

fn assert_close(value: &serde_json::Value, expected: &[f32]) {
    let actual: Vec<f32> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_f64().expect("number") as f32)
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-4, "{actual:?} != {expected:?}");
    }
}

fn node_of_bone() -> BTreeMap<String, u32> {
    BONE_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| (name.to_string(), index as u32 + 10))
        .collect()
}

fn joint(bone: &str, force: SidecarJointForce) -> SidecarSpringJoint {
    SidecarSpringJoint {
        bone: bone.to_string(),
        force,
        drag_force: 0.4,
        gravity_dir: [0.0, -1.0, 0.0],
        hit_radius: 0.02,
    }
}

fn spring() -> SidecarSpring {
    let rest_relative = SidecarJointForce::RestRelative {
        bending_relaxation: 0.06,
        gravity_scale: 1.0,
    };
    SidecarSpring {
        collision_mode: SidecarCollisionMode::SegmentExtended,
        colliders: vec![
            SidecarCollider {
                name: "Head#0".to_string(),
                bone: "Head".to_string(),
                offset: [0.0, 0.05, 0.02],
                shape: SidecarColliderShape::Sphere,
                radius: 0.08,
                tail: None,
                origin: SidecarColliderOrigin::Authored,
            },
            SidecarCollider {
                name: "Hips#1".to_string(),
                bone: "Hips".to_string(),
                offset: [0.0, 0.1, 0.0],
                shape: SidecarColliderShape::Capsule,
                radius: 0.04,
                tail: Some([0.0, -0.25, 0.0]),
                origin: SidecarColliderOrigin::Authored,
            },
        ],
        collider_groups: vec![
            SidecarColliderGroup {
                name: "head".to_string(),
                colliders: vec!["Head#0".to_string()],
            },
            SidecarColliderGroup {
                name: "body".to_string(),
                colliders: vec!["Hips#1".to_string()],
            },
        ],
        chains: vec![
            SidecarSpringChain {
                name: "hair".to_string(),
                center: Some("Head".to_string()),
                collider_groups: vec!["head".to_string(), "body".to_string()],
                joints: vec![
                    joint("Hair1", rest_relative),
                    joint("Hair2", rest_relative),
                    joint("Hair3", rest_relative),
                ],
                tail: Some("Hair3".to_string()),
                enabled: true,
                origin: None,
            },
            SidecarSpringChain {
                name: "disabled".to_string(),
                center: None,
                collider_groups: vec![],
                joints: vec![joint(
                    "Hair1",
                    SidecarJointForce::Vrm {
                        stiffness: 1.0,
                        gravity_power: 0.0,
                    },
                )],
                tail: None,
                enabled: false,
                origin: None,
            },
        ],
    }
}

fn sidecar() -> AvatarSidecar {
    let humanoid = [("Hips", "Hips"), ("Head", "Head")]
        .into_iter()
        .map(|(role, bone)| (role.to_string(), bone.to_string()))
        .collect();
    AvatarSidecar {
        schema: SIDECAR_SCHEMA_VERSION,
        humanoid,
        visemes: BTreeMap::new(),
        blink: SidecarBlink {
            both: None,
            left: None,
            right: None,
        },
        expression_mesh: None,
        expressions: vec![],
        spring: spring(),
        materials: BTreeMap::new(),
        submesh_roles: vec![],
        hidden_triangles: vec![],
        stats: AvatarStats::default(),
    }
}

fn extensions() -> VrmExtensions {
    let sidecar = sidecar();
    let bones = bones();
    let frames = centimetre_frames();
    let node_of_bone = node_of_bone();
    let meta = VrmMeta::new("test avatar", "tester");
    build_vrm_extensions(VrmInput {
        sidecar: &sidecar,
        bones: &bones,
        frames: &frames,
        node_of_bone: &node_of_bone,
        meta: &meta,
    })
    .expect("extensions build")
}

#[test]
fn extensions_are_listed_in_standard_then_vendor_order() {
    assert_eq!(
        extensions().names(),
        [VRMC_VRM, VRMC_SPRING_BONE, THYLLORE_AVATAR]
    );
}

#[test]
fn humanoid_bones_use_vrm_names_and_gltf_nodes() {
    let extensions = extensions();
    let vrm = extensions.get(VRMC_VRM).expect("VRMC_vrm");

    assert_eq!(vrm["specVersion"], "1.0");
    assert_eq!(vrm["humanoid"]["humanBones"]["hips"]["node"], 10);
    assert_eq!(vrm["humanoid"]["humanBones"]["head"]["node"], 11);
    assert_eq!(vrm["meta"]["name"], "test avatar");
    assert_eq!(vrm["meta"]["authors"][0], "tester");
    assert_eq!(vrm["meta"]["licenseUrl"], VRM_1_0_LICENSE_URL);
    assert_eq!(vrm["meta"]["avatarPermission"], "onlyAuthor");
    assert_eq!(vrm["meta"]["modification"], "prohibited");
}

#[test]
fn colliders_are_written_in_node_space() {
    let extensions = extensions();
    let spring_bone = extensions.get(VRMC_SPRING_BONE).expect("VRMC_springBone");

    let sphere = &spring_bone["colliders"][0];
    assert_eq!(sphere["node"], 11);
    assert_close(&sphere["shape"]["sphere"]["offset"], &[0.0, 5.0, 2.0]);
    assert!((sphere["shape"]["sphere"]["radius"].as_f64().unwrap() - 8.0).abs() < 1e-5);
    let capsule = &spring_bone["colliders"][1];
    assert_eq!(capsule["node"], 10);
    assert_close(&capsule["shape"]["capsule"]["tail"], &[0.0, -25.0, 0.0]);
    assert_eq!(
        spring_bone["colliderGroups"][1]["colliders"],
        serde_json::json!([1])
    );
}

#[test]
fn rest_relative_joints_carry_strand_relative_stiffness_and_no_gravity() {
    let extensions = extensions();
    let spring_bone = extensions.get(VRMC_SPRING_BONE).expect("VRMC_springBone");
    let springs = spring_bone["springs"].as_array().expect("springs");

    assert_eq!(springs.len(), 1, "a disabled chain is not written");
    let hair = &springs[0];
    assert_eq!(hair["name"], "hair");
    assert_eq!(hair["center"], 11);
    assert_eq!(hair["colliderGroups"], serde_json::json!([0, 1]));
    let joints = hair["joints"].as_array().expect("joints");
    assert_eq!(joints.len(), 3);
    let strand_length: f64 = 0.1 + 0.1;
    let expected_stiffness = 0.06 / strand_length;
    for joint in joints {
        assert!((joint["stiffness"].as_f64().unwrap() - expected_stiffness).abs() < 1e-5);
        assert_eq!(joint["gravityPower"], 0.0);
        assert!((joint["hitRadius"].as_f64().unwrap() - 2.0).abs() < 1e-5);
    }
}

#[test]
fn importer_reads_back_the_written_spring_bone() {
    let extensions = extensions();
    let spring_bone = extensions.get(VRMC_SPRING_BONE).expect("VRMC_springBone");
    let resolve = |node: u32| -> Option<u32> { node.checked_sub(10) };

    let setup = parse_vrmc_spring_bone(spring_bone, &resolve).expect("importer parses");

    assert_eq!(setup.colliders[0].bone_id, 1);
    assert_eq!(setup.colliders[1].bone_id, 0);
    assert!(matches!(
        setup.colliders[1].shape,
        ColliderShape::Capsule { .. }
    ));
    let chain = &setup.chains[0];
    assert_eq!(chain.name, "hair");
    assert_eq!(chain.center_bone_id, Some(1));
    assert_eq!(chain.tail_bone_id, Some(4));
    assert_eq!(chain.joints.len(), 2);
    assert!(matches!(
        chain.joints[0].force,
        JointForce::Vrm(VrmJointForce { gravity_power, .. }) if gravity_power == 0.0
    ));
}

#[test]
fn vendor_extension_round_trips_the_sidecar() {
    let extensions = extensions();
    let value = extensions.get(THYLLORE_AVATAR).expect("THYLLORE_avatar");

    let restored: AvatarSidecar = serde_json::from_value(value.clone()).expect("sidecar parses");

    assert_eq!(restored, sidecar());
}

#[test]
fn unmapped_bone_is_an_error() {
    let sidecar = sidecar();
    let bones = bones();
    let frames = centimetre_frames();
    let mut node_of_bone = node_of_bone();
    node_of_bone.remove("Hair2");
    let meta = VrmMeta::new("test avatar", "tester");

    let error = build_vrm_extensions(VrmInput {
        sidecar: &sidecar,
        bones: &bones,
        frames: &frames,
        node_of_bone: &node_of_bone,
        meta: &meta,
    })
    .expect_err("missing node");

    assert!(error.to_string().contains("Hair2"));
}

#[test]
fn meta_without_author_is_rejected() {
    let mut meta = VrmMeta::new("test avatar", "");
    assert!(meta.validate().is_err());
    meta.authors = vec!["tester".to_string()];
    assert!(meta.validate().is_ok());
}
