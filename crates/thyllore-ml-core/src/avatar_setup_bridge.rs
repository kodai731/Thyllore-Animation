use std::collections::{BTreeMap, HashSet};

use crate::param_rows::{param_rows, ParamRow};
use thyllore_anim_core::spring_bone::{
    SpringJointParam, SPRING_JOINT_SCALAR_PARAMS, SPRING_JOINT_UI_PARAMS,
};
use thyllore_avatar_core::expression::components::preset::ExpressionLibrary;
use thyllore_avatar_core::garment::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::stats::components::stats::AvatarStats;
use thyllore_avatar_core::vrchat::sidecar::{
    build_hybrid_bone_map, build_sidecar, AvatarSidecar, SidecarHiddenTriangles, SidecarInput,
    SidecarSubmeshRole,
};
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar, SidecarSpring, SidecarSpringChain,
};

pub fn spring_joint_params() -> Vec<ParamRow> {
    param_rows(
        &SPRING_JOINT_UI_PARAMS,
        &SPRING_JOINT_SCALAR_PARAMS,
        &SpringJointParam::default(),
    )
}

#[derive(Clone, serde::Deserialize)]
pub struct BoneSpec {
    pub name: String,
    pub parent: Option<usize>,
    pub rest_position: [f32; 3],
}

impl From<BoneSpec> for BoneInput {
    fn from(spec: BoneSpec) -> Self {
        BoneInput {
            name: spec.name,
            parent: spec.parent,
            rest_position: spec.rest_position,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct SidecarBuildInput {
    pub bones: Vec<BoneSpec>,
    pub humanoid_overrides: BTreeMap<String, String>,
    pub spring: SidecarSpring,
    pub submesh_roles: Vec<SidecarSubmeshRole>,
    pub hidden_triangles: Vec<SidecarHiddenTriangles>,
    pub materials: BTreeMap<String, String>,
    pub stats: AvatarStats,
}

pub fn validate_sidecar_spring(spring: &SidecarSpring, bones: &[BoneInput]) -> anyhow::Result<()> {
    let bone_names: HashSet<&str> = bones.iter().map(|b| b.name.as_str()).collect();

    for (i, chain) in spring.chains.iter().enumerate() {
        validate_chain(i, chain, &bone_names, spring)?;
    }

    for (i, group) in spring.collider_groups.iter().enumerate() {
        let collider_names: HashSet<&str> =
            spring.colliders.iter().map(|c| c.name.as_str()).collect();
        for collider_name in &group.colliders {
            if !collider_names.contains(collider_name.as_str()) {
                anyhow::bail!(
                    "collider group {} (name={:?}) references unknown collider {:?}",
                    i,
                    group.name,
                    collider_name
                );
            }
        }
    }

    for (i, collider) in spring.colliders.iter().enumerate() {
        if !bone_names.contains(collider.bone.as_str()) {
            anyhow::bail!(
                "collider {} (name={:?}) references unknown bone {:?}",
                i,
                collider.name,
                collider.bone
            );
        }
    }

    Ok(())
}

fn validate_chain(
    i: usize,
    chain: &SidecarSpringChain,
    bone_names: &HashSet<&str>,
    spring: &SidecarSpring,
) -> anyhow::Result<()> {
    if chain.joints.is_empty() {
        anyhow::bail!("chain {} (name={:?}) has no joints", i, chain.name);
    }
    for (j, joint) in chain.joints.iter().enumerate() {
        if !bone_names.contains(joint.bone.as_str()) {
            anyhow::bail!(
                "chain {} (name={:?}) joint {} references unknown bone {:?}",
                i,
                chain.name,
                j,
                joint.bone
            );
        }
    }
    if let Some(ref center) = chain.center {
        if !bone_names.contains(center.as_str()) {
            anyhow::bail!(
                "chain {} (name={:?}) references unknown center bone {:?}",
                i,
                chain.name,
                center
            );
        }
    }
    if let Some(ref tail) = chain.tail {
        if !bone_names.contains(tail.as_str()) {
            anyhow::bail!(
                "chain {} (name={:?}) references unknown tail bone {:?}",
                i,
                chain.name,
                tail
            );
        }
    }
    for group_name in &chain.collider_groups {
        if !spring.collider_groups.iter().any(|g| g.name == *group_name) {
            anyhow::bail!(
                "chain {} (name={:?}) references unknown collider group {:?}",
                i,
                chain.name,
                group_name
            );
        }
    }
    Ok(())
}

pub fn sidecar_build(input: &SidecarBuildInput) -> anyhow::Result<String> {
    let bones: Vec<BoneInput> = input.bones.iter().cloned().map(BoneInput::from).collect();

    validate_sidecar_spring(&input.spring, &bones)?;

    let setup = spring_setup_from_sidecar(&input.spring, &bones);
    let normalized_spring = build_sidecar_spring(&setup, &bones);

    let (mapping, _) = infer_mapping(&bones);
    let mut sidecar = build_sidecar(SidecarInput {
        mapping: &mapping,
        bones: &bones,
        channel_names: &[],
        expression_mesh_name: None,
        library: &ExpressionLibrary::default(),
        spring: &normalized_spring,
        materials: &input.materials,
        submesh_roles: &submesh_role_map(&input.submesh_roles),
        hidden_triangles: &hidden_triangle_map(&input.hidden_triangles),
        stats: &input.stats,
    });

    apply_humanoid_overrides(&mut sidecar.humanoid, &input.humanoid_overrides, &bones)?;

    serde_json::to_string_pretty(&sidecar).map_err(|e| anyhow::anyhow!(e))
}

fn submesh_role_map(roles: &[SidecarSubmeshRole]) -> BTreeMap<SubmeshKey, SubmeshRole> {
    let mut map = BTreeMap::new();
    for entry in roles {
        map.insert(
            SubmeshKey {
                mesh: entry.mesh.clone(),
                material: entry.material.clone(),
            },
            entry.role,
        );
    }
    map
}

fn hidden_triangle_map(hidden: &[SidecarHiddenTriangles]) -> BTreeMap<SubmeshKey, Vec<u32>> {
    let mut map = BTreeMap::new();
    for entry in hidden {
        map.insert(
            SubmeshKey {
                mesh: entry.mesh.clone(),
                material: entry.material.clone(),
            },
            entry.triangles.clone(),
        );
    }
    map
}

fn apply_humanoid_overrides(
    humanoid: &mut BTreeMap<String, String>,
    overrides: &BTreeMap<String, String>,
    bones: &[BoneInput],
) -> anyhow::Result<()> {
    for (unity_name, bone_name) in overrides {
        if !bones.iter().any(|b| b.name == *bone_name) {
            anyhow::bail!(
                "humanoid override {:?} references unknown bone {:?}",
                unity_name,
                bone_name
            );
        }
        humanoid.insert(unity_name.clone(), bone_name.clone());
    }
    Ok(())
}

pub fn sidecar_read(json: &str) -> anyhow::Result<AvatarSidecar> {
    serde_json::from_str(json).map_err(|e| anyhow::anyhow!(e))
}

#[derive(serde::Serialize)]
pub struct HumanoidMappingOut {
    pub mapping: std::collections::BTreeMap<String, String>,
    pub unresolved: Vec<String>,
}

pub fn infer_humanoid_mapping(bones: &[BoneInput]) -> HumanoidMappingOut {
    let (mapping, unresolved) = infer_mapping(bones);
    let map = build_hybrid_bone_map(&mapping, bones);
    let unresolved_names: Vec<String> = unresolved
        .iter()
        .map(|u| u.role.unity_name().to_string())
        .collect();
    HumanoidMappingOut {
        mapping: map,
        unresolved: unresolved_names,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::param_rows::ParamDefault;
    use thyllore_avatar_core::vrchat::sidecar::SidecarBlink;
    use thyllore_avatar_core::vrchat::sidecar_spring::{
        SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin, SidecarColliderShape,
        SidecarSpringChain, SidecarSpringJoint,
    };

    #[test]
    fn avatar_setup_bridge_spring_joint_params_names_match() {
        let params = spring_joint_params();
        let names: Vec<&str> = params.iter().map(|p| p.name).collect();
        let expected: Vec<&str> = SPRING_JOINT_UI_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn avatar_setup_bridge_spring_joint_params_carry_defaults() {
        let params = spring_joint_params();
        let gravity_dir = params.iter().find(|p| p.name == "gravity_dir").unwrap();
        assert_eq!(gravity_dir.default, ParamDefault::Vector([0.0, -1.0, 0.0]));
        let stiffness = params.iter().find(|p| p.name == "stiffness").unwrap();
        assert_eq!(
            stiffness.default,
            ParamDefault::Scalar(SpringJointParam::default().stiffness)
        );
    }

    #[test]
    fn avatar_setup_bridge_infer_humanoid_mapping_resolves_hips() {
        let bones: Vec<BoneInput> = vec![
            BoneInput {
                name: "Hips".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
            },
            BoneInput {
                name: "Spine".into(),
                parent: Some(0),
                rest_position: [0.0, 1.1, 0.0],
            },
            BoneInput {
                name: "Head".into(),
                parent: Some(1),
                rest_position: [0.0, 1.2, 0.0],
            },
        ];
        let result = infer_humanoid_mapping(&bones);
        assert!(
            result.mapping.contains_key("Hips"),
            "Hips should be resolved"
        );
    }

    #[test]
    fn avatar_setup_bridge_sidecar_build_roundtrip_preserves_spring() {
        let input = SidecarBuildInput {
            bones: vec![
                BoneSpec {
                    name: "Bone0".into(),
                    parent: None,
                    rest_position: [0.0, 1.0, 0.0],
                },
                BoneSpec {
                    name: "Bone1".into(),
                    parent: Some(0),
                    rest_position: [0.0, 1.1, 0.0],
                },
                BoneSpec {
                    name: "Bone2".into(),
                    parent: Some(1),
                    rest_position: [0.0, 1.2, 0.0],
                },
            ],
            humanoid_overrides: BTreeMap::new(),
            spring: SidecarSpring {
                collision_mode: Default::default(),
                force_model: Default::default(),
                colliders: vec![SidecarCollider {
                    name: "collider0".into(),
                    bone: "Bone0".into(),
                    offset: [0.0, 0.0, 0.0],
                    radius: 0.1,
                    shape: SidecarColliderShape::Sphere,
                    tail: None,
                    origin: SidecarColliderOrigin::Authored,
                }],
                collider_groups: vec![SidecarColliderGroup {
                    name: "group0".into(),
                    colliders: vec!["collider0".into()],
                }],
                chains: vec![SidecarSpringChain {
                    name: "chain0".into(),
                    joints: vec![
                        SidecarSpringJoint {
                            bone: "Bone0".into(),
                            stiffness: 0.5,
                            drag_force: 0.1,
                            gravity_power: 1.0,
                            gravity_dir: [0.0, -1.0, 0.0],
                            hit_radius: 0.0,
                        },
                        SidecarSpringJoint {
                            bone: "Bone1".into(),
                            stiffness: 0.5,
                            drag_force: 0.1,
                            gravity_power: 1.0,
                            gravity_dir: [0.0, -1.0, 0.0],
                            hit_radius: 0.0,
                        },
                    ],
                    collider_groups: vec!["group0".into()],
                    center: None,
                    tail: None,
                    origin: Default::default(),
                }],
            },
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            materials: BTreeMap::new(),
            stats: AvatarStats::default(),
        };

        let json = sidecar_build(&input).unwrap();
        let parsed = sidecar_read(&json).unwrap();
        assert_eq!(parsed.spring.chains.len(), 1);
        assert_eq!(parsed.spring.chains[0].name, "chain0");
        assert_eq!(parsed.spring.chains[0].joints.len(), 3);
        assert_eq!(parsed.spring.colliders.len(), 1);
        assert_eq!(parsed.spring.collider_groups.len(), 1);
    }

    #[test]
    fn avatar_setup_bridge_validate_unknown_joint_bone() {
        let input = SidecarBuildInput {
            bones: vec![BoneSpec {
                name: "Bone0".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
            }],
            humanoid_overrides: BTreeMap::new(),
            spring: SidecarSpring {
                collision_mode: Default::default(),
                force_model: Default::default(),
                colliders: Vec::new(),
                collider_groups: Vec::new(),
                chains: vec![SidecarSpringChain {
                    name: "chain0".into(),
                    joints: vec![SidecarSpringJoint {
                        bone: "UnknownBone".into(),
                        stiffness: 0.5,
                        drag_force: 0.1,
                        gravity_power: 1.0,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    }],
                    collider_groups: Vec::new(),
                    center: None,
                    tail: None,
                    origin: Default::default(),
                }],
            },
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            materials: BTreeMap::new(),
            stats: AvatarStats::default(),
        };

        let err = sidecar_build(&input).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("UnknownBone"),
            "error should mention the unknown bone name, got: {}",
            msg
        );
    }

    #[test]
    fn avatar_setup_bridge_validate_unknown_collider_group() {
        let input = SidecarBuildInput {
            bones: vec![BoneSpec {
                name: "Bone0".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
            }],
            humanoid_overrides: BTreeMap::new(),
            spring: SidecarSpring {
                collision_mode: Default::default(),
                force_model: Default::default(),
                colliders: Vec::new(),
                collider_groups: Vec::new(),
                chains: vec![SidecarSpringChain {
                    name: "chain0".into(),
                    joints: vec![SidecarSpringJoint {
                        bone: "Bone0".into(),
                        stiffness: 0.5,
                        drag_force: 0.1,
                        gravity_power: 1.0,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    }],
                    collider_groups: vec!["unknown_group".into()],
                    center: None,
                    tail: None,
                    origin: Default::default(),
                }],
            },
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            materials: BTreeMap::new(),
            stats: AvatarStats::default(),
        };

        let err = sidecar_build(&input).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown_group"),
            "error should mention the unknown group name, got: {}",
            msg
        );
    }

    #[test]
    fn avatar_setup_bridge_sidecar_read_roundtrip() {
        let sidecar = AvatarSidecar {
            schema: 1,
            humanoid: std::collections::BTreeMap::new(),
            visemes: std::collections::BTreeMap::new(),
            blink: SidecarBlink {
                both: None,
                left: None,
                right: None,
            },
            expression_mesh: None,
            expressions: Vec::new(),
            spring: Default::default(),
            materials: std::collections::BTreeMap::new(),
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            stats: Default::default(),
        };
        let json = serde_json::to_string(&sidecar).unwrap();
        let parsed = sidecar_read(&json).unwrap();
        assert_eq!(parsed.schema, 1);
    }
}
