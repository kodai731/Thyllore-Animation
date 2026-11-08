use std::collections::{BTreeMap, HashSet};

use crate::blender_space::{column_major, MatrixRows};
use crate::param_rows::{param_rows, ParamRow};
use glam::Mat4;
use thyllore_anim_core::spring_bone::{
    RestRelativeJointForce, SpringForceModel, SpringJointParam, VrmJointForce,
    REST_RELATIVE_JOINT_FORCE_SCALAR_PARAMS, REST_RELATIVE_JOINT_FORCE_UI_PARAMS,
    SPRING_JOINT_SCALAR_PARAMS, SPRING_JOINT_UI_PARAMS, VRM_JOINT_FORCE_SCALAR_PARAMS,
    VRM_JOINT_FORCE_UI_PARAMS,
};
use thyllore_avatar_core::cloth_setup::components::chain_gen::RootAttachment;
use thyllore_avatar_core::cloth_setup::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::expression::components::preset::ExpressionLibrary;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::stats::components::stats::AvatarStats;
use thyllore_avatar_core::vrchat::sidecar::{
    build_hybrid_bone_map, build_sidecar, AvatarSidecar, SidecarHiddenTriangles, SidecarInput,
    SidecarSubmeshRole,
};
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, map_sidecar_colliders, spring_setup_from_sidecar, BoneRestFrame,
    ColliderSpace, SidecarSpring, SidecarSpringChain,
};

pub fn spring_joint_params() -> Vec<ParamRow> {
    param_rows(
        &SPRING_JOINT_UI_PARAMS,
        &SPRING_JOINT_SCALAR_PARAMS,
        &SpringJointParam::default(),
    )
}

#[derive(serde::Serialize)]
pub struct ForceModelParams {
    pub model: &'static str,
    pub label: &'static str,
    pub is_default: bool,
    pub params: Vec<ParamRow>,
}

/// The magnitude rows of every force model, in the engine's model order.
pub fn spring_joint_force_params() -> Vec<ForceModelParams> {
    SpringForceModel::ALL
        .iter()
        .map(|model| ForceModelParams {
            model: model.key(),
            label: model.label(),
            is_default: *model == SpringForceModel::default(),
            params: match model {
                SpringForceModel::Vrm => param_rows(
                    &VRM_JOINT_FORCE_UI_PARAMS,
                    &VRM_JOINT_FORCE_SCALAR_PARAMS,
                    &VrmJointForce::default(),
                ),
                SpringForceModel::RestRelative => param_rows(
                    &REST_RELATIVE_JOINT_FORCE_UI_PARAMS,
                    &REST_RELATIVE_JOINT_FORCE_SCALAR_PARAMS,
                    &RestRelativeJointForce::default(),
                ),
            },
        })
        .collect()
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BoneSpec {
    pub name: String,
    pub parent: Option<usize>,
    pub rest_position: [f32; 3],
    /// Row-major bone-local to model-space (engine axes, metres) rest transform, as the engine binds
    /// skins; absent for a bone no skin is bound to
    #[serde(default)]
    pub bind_matrix: Option<MatrixRows>,
}

impl BoneSpec {
    pub fn bind_global(&self) -> anyhow::Result<Mat4> {
        let rows = self
            .bind_matrix
            .ok_or_else(|| anyhow::anyhow!("bone {:?} has no bind_matrix", self.name))?;
        Ok(column_major(&rows))
    }
}

/// The sidecar frame of each bone: the axes of its bind matrix, whose lengths are metres per bone unit.
pub(crate) fn rest_frames(bones: &[BoneSpec]) -> anyhow::Result<Vec<BoneRestFrame>> {
    bones
        .iter()
        .map(|bone| {
            BoneRestFrame::from_global_transform(&bone.bind_global()?, 1.0)
                .ok_or_else(|| anyhow::anyhow!("bone {:?} has a singular bind_matrix", bone.name))
        })
        .collect()
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpringCollidersRequest {
    pub spring: SidecarSpring,
    pub bones: Vec<BoneSpec>,
}

/// The spring with its colliders moved from the sidecar's model space into each bone's rest frame.
pub fn spring_colliders_to_local(
    request: &SpringCollidersRequest,
) -> anyhow::Result<SidecarSpring> {
    map_spring_colliders(&request.spring, &request.bones, ColliderSpace::BoneLocal)
}

/// The inverse of `spring_colliders_to_local`: colliders given in bone rest frames, written in model space.
pub fn spring_colliders_to_model(
    request: &SpringCollidersRequest,
) -> anyhow::Result<SidecarSpring> {
    map_spring_colliders(&request.spring, &request.bones, ColliderSpace::Model)
}

fn map_spring_colliders(
    spring: &SidecarSpring,
    bones: &[BoneSpec],
    target: ColliderSpace,
) -> anyhow::Result<SidecarSpring> {
    let frames = rest_frames(bones)?;
    let bone_inputs: Vec<BoneInput> = bones.iter().cloned().map(BoneInput::from).collect();
    Ok(map_sidecar_colliders(spring, &bone_inputs, &frames, target))
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

pub(crate) fn bone_inputs(specs: &[BoneSpec]) -> Vec<BoneInput> {
    specs.iter().cloned().map(Into::into).collect()
}

#[derive(serde::Deserialize)]
pub struct SidecarBuildInput {
    /// The sidecar the setup was loaded from; its fields the addon does not edit are kept
    #[serde(default)]
    pub base: Option<AvatarSidecar>,
    pub bones: Vec<BoneSpec>,
    pub humanoid_overrides: BTreeMap<String, String>,
    pub spring: SidecarSpring,
    pub submesh_roles: Vec<SidecarSubmeshRole>,
    pub hidden_triangles: Vec<SidecarHiddenTriangles>,
    pub materials: BTreeMap<String, String>,
    pub stats: MeshStats,
}

/// The counts only Blender knows; the bone and spring counts come from the build input itself.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct MeshStats {
    pub triangles: u32,
    pub materials: u32,
    pub meshes: u32,
    pub skinned_meshes: u32,
    pub morph_meshes: u32,
}

fn avatar_stats(mesh: &MeshStats, bone_count: usize, spring: &SidecarSpring) -> AvatarStats {
    AvatarStats {
        triangles: mesh.triangles,
        bones: bone_count as u32,
        materials: mesh.materials,
        skinned_meshes: mesh.skinned_meshes,
        meshes: mesh.meshes,
        morph_meshes: mesh.morph_meshes,
        spring_chains: spring.chains.len() as u32,
        spring_transforms: spring
            .chains
            .iter()
            .map(|chain| chain.joints.len() as u32)
            .sum(),
        spring_colliders: spring.colliders.len() as u32,
        texture_bytes: 0,
    }
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

    let frames = BoneRestFrame::identity_for(bones.len());
    let setup = spring_setup_from_sidecar(&input.spring, &bones, &frames);
    let normalized_spring = build_sidecar_spring(&setup, &bones, &frames);

    let (mapping, _) = infer_mapping(&bones);
    let stats = avatar_stats(&input.stats, bones.len(), &normalized_spring);
    let fresh = build_sidecar(SidecarInput {
        mapping: &mapping,
        bones: &bones,
        channel_names: &[],
        expression_mesh_name: None,
        library: &ExpressionLibrary::default(),
        spring: &normalized_spring,
        materials: &input.materials,
        submesh_roles: &submesh_role_map(&input.submesh_roles),
        submesh_attachments: &submesh_attachment_map(&input.submesh_roles),
        hidden_triangles: &hidden_triangle_map(&input.hidden_triangles),
        stats: &stats,
    });
    let mut sidecar = match &input.base {
        Some(base) => AvatarSidecar {
            schema: fresh.schema,
            humanoid: fresh.humanoid,
            spring: fresh.spring,
            submesh_roles: fresh.submesh_roles,
            hidden_triangles: fresh.hidden_triangles,
            stats: fresh.stats,
            materials: if input.materials.is_empty() {
                base.materials.clone()
            } else {
                fresh.materials
            },
            visemes: base.visemes.clone(),
            blink: base.blink.clone(),
            expression_mesh: base.expression_mesh.clone(),
            expressions: base.expressions.clone(),
        },
        None => fresh,
    };

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

fn submesh_attachment_map(roles: &[SidecarSubmeshRole]) -> BTreeMap<SubmeshKey, RootAttachment> {
    roles
        .iter()
        .filter_map(|entry| {
            Some((
                SubmeshKey {
                    mesh: entry.mesh.clone(),
                    material: entry.material.clone(),
                },
                entry.attachment?,
            ))
        })
        .collect()
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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferHumanoidMappingRequest {
    pub bones: Vec<BoneSpec>,
}

pub fn infer_humanoid_mapping(request: &InferHumanoidMappingRequest) -> HumanoidMappingOut {
    let bones = bone_inputs(&request.bones);
    let (mapping, unresolved) = infer_mapping(&bones);
    let map = build_hybrid_bone_map(&mapping, &bones);
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
    const IDENTITY_ROWS: MatrixRows = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    use super::*;

    use crate::param_rows::ParamDefault;
    use thyllore_avatar_core::vrchat::sidecar::SidecarBlink;
    use thyllore_avatar_core::vrchat::sidecar_spring::{
        SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin, SidecarColliderShape,
        SidecarJointForce, SidecarSpringChain, SidecarSpringJoint,
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
        let drag_force = params.iter().find(|p| p.name == "drag_force").unwrap();
        assert_eq!(
            drag_force.default,
            ParamDefault::Scalar(SpringJointParam::default().drag_force)
        );
    }

    #[test]
    fn avatar_setup_bridge_infer_humanoid_mapping_resolves_hips() {
        let bones = vec![
            BoneSpec {
                name: "Hips".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
                bind_matrix: None,
            },
            BoneSpec {
                name: "Spine".into(),
                parent: Some(0),
                rest_position: [0.0, 1.1, 0.0],
                bind_matrix: None,
            },
            BoneSpec {
                name: "Head".into(),
                parent: Some(1),
                rest_position: [0.0, 1.2, 0.0],
                bind_matrix: None,
            },
        ];
        let result = infer_humanoid_mapping(&InferHumanoidMappingRequest { bones });
        assert!(
            result.mapping.contains_key("Hips"),
            "Hips should be resolved"
        );
    }

    fn build_input() -> SidecarBuildInput {
        SidecarBuildInput {
            base: None,
            bones: vec![
                BoneSpec {
                    name: "Bone0".into(),
                    parent: None,
                    rest_position: [0.0, 1.0, 0.0],
                    bind_matrix: None,
                },
                BoneSpec {
                    name: "Bone1".into(),
                    parent: Some(0),
                    rest_position: [0.0, 1.1, 0.0],
                    bind_matrix: None,
                },
                BoneSpec {
                    name: "Bone2".into(),
                    parent: Some(1),
                    rest_position: [0.0, 1.2, 0.0],
                    bind_matrix: None,
                },
            ],
            humanoid_overrides: BTreeMap::new(),
            spring: SidecarSpring {
                collision_mode: Default::default(),
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
                            force: SidecarJointForce::Vrm {
                                stiffness: 0.5,
                                gravity_power: 1.0,
                            },
                            drag_force: 0.1,
                            gravity_dir: [0.0, -1.0, 0.0],
                            hit_radius: 0.0,
                        },
                        SidecarSpringJoint {
                            bone: "Bone1".into(),
                            force: SidecarJointForce::Vrm {
                                stiffness: 0.5,
                                gravity_power: 1.0,
                            },
                            drag_force: 0.1,
                            gravity_dir: [0.0, -1.0, 0.0],
                            hit_radius: 0.0,
                        },
                    ],
                    collider_groups: vec!["group0".into()],
                    center: None,
                    tail: None,
                    enabled: true,
                    origin: Default::default(),
                }],
            },
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            materials: BTreeMap::new(),
            stats: MeshStats::default(),
        }
    }

    #[test]
    fn avatar_setup_bridge_sidecar_build_roundtrip_preserves_spring() {
        let input = build_input();

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
            base: None,
            bones: vec![BoneSpec {
                name: "Bone0".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
                bind_matrix: None,
            }],
            humanoid_overrides: BTreeMap::new(),
            spring: SidecarSpring {
                collision_mode: Default::default(),
                colliders: Vec::new(),
                collider_groups: Vec::new(),
                chains: vec![SidecarSpringChain {
                    name: "chain0".into(),
                    joints: vec![SidecarSpringJoint {
                        bone: "UnknownBone".into(),
                        force: SidecarJointForce::Vrm {
                            stiffness: 0.5,
                            gravity_power: 1.0,
                        },
                        drag_force: 0.1,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    }],
                    collider_groups: Vec::new(),
                    center: None,
                    tail: None,
                    enabled: true,
                    origin: Default::default(),
                }],
            },
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            materials: BTreeMap::new(),
            stats: MeshStats::default(),
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
            base: None,
            bones: vec![BoneSpec {
                name: "Bone0".into(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
                bind_matrix: None,
            }],
            humanoid_overrides: BTreeMap::new(),
            spring: SidecarSpring {
                collision_mode: Default::default(),
                colliders: Vec::new(),
                collider_groups: Vec::new(),
                chains: vec![SidecarSpringChain {
                    name: "chain0".into(),
                    joints: vec![SidecarSpringJoint {
                        bone: "Bone0".into(),
                        force: SidecarJointForce::Vrm {
                            stiffness: 0.5,
                            gravity_power: 1.0,
                        },
                        drag_force: 0.1,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    }],
                    collider_groups: vec!["unknown_group".into()],
                    center: None,
                    tail: None,
                    enabled: true,
                    origin: Default::default(),
                }],
            },
            submesh_roles: Vec::new(),
            hidden_triangles: Vec::new(),
            materials: BTreeMap::new(),
            stats: MeshStats::default(),
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

    fn framed_bones() -> Vec<BoneSpec> {
        vec![
            BoneSpec {
                name: "Hips".into(),
                parent: None,
                rest_position: [0.0, 0.0, 0.0],
                bind_matrix: Some(IDENTITY_ROWS),
            },
            BoneSpec {
                name: "Head".into(),
                parent: Some(0),
                rest_position: [0.0, 1.0, 0.0],
                bind_matrix: Some([
                    [0.0, -2.0, 0.0, 0.0],
                    [2.0, 0.0, 0.0, 1.0],
                    [0.0, 0.0, 2.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ]),
            },
        ]
    }

    fn model_space_spring() -> SidecarSpring {
        SidecarSpring {
            colliders: vec![SidecarCollider {
                name: "head".into(),
                bone: "Head".into(),
                offset: [0.0, 2.0, 0.0],
                shape: SidecarColliderShape::Capsule,
                radius: 1.0,
                tail: Some([-4.0, 0.0, 0.0]),
                origin: SidecarColliderOrigin::Authored,
            }],
            ..SidecarSpring::default()
        }
    }

    #[test]
    fn colliders_round_trip_between_model_space_and_bone_frames() {
        let bones = framed_bones();
        let local = spring_colliders_to_local(&SpringCollidersRequest {
            spring: model_space_spring(),
            bones: bones.clone(),
        })
        .unwrap();
        let collider = &local.colliders[0];
        assert!((collider.offset[0] - 1.0).abs() < 1e-5 && collider.offset[1].abs() < 1e-5);
        assert!((collider.radius - 0.5).abs() < 1e-5);
        let tail = collider.tail.unwrap();
        assert!((tail[1] - 2.0).abs() < 1e-5 && tail[0].abs() < 1e-5);

        let model = spring_colliders_to_model(&SpringCollidersRequest {
            spring: local,
            bones,
        })
        .unwrap();
        assert_eq!(model, model_space_spring());
    }

    #[test]
    fn colliders_need_a_bind_matrix_on_every_bone() {
        let mut bones = framed_bones();
        bones[1].bind_matrix = None;
        let error = spring_colliders_to_local(&SpringCollidersRequest {
            spring: model_space_spring(),
            bones,
        })
        .unwrap_err();
        assert!(error.to_string().contains("bind_matrix"));
    }

    #[test]
    fn sidecar_build_keeps_the_base_fields_the_addon_does_not_edit() {
        let mut base = sidecar_read(&sidecar_build(&build_input()).unwrap()).unwrap();
        base.expressions
            .push(thyllore_avatar_core::vrchat::sidecar::SidecarExpression {
                name: "smile".into(),
                weights: BTreeMap::from([("mouth_smile".to_string(), 1.0)]),
            });
        base.materials
            .insert("Body".into(), "textures/body.png".into());
        base.visemes.insert("aa".into(), "vrc.v_aa".into());

        let mut input = build_input();
        input.base = Some(base.clone());
        let rebuilt = sidecar_read(&sidecar_build(&input).unwrap()).unwrap();

        assert_eq!(rebuilt.expressions, base.expressions);
        assert_eq!(rebuilt.materials, base.materials);
        assert_eq!(rebuilt.visemes, base.visemes);
        assert_eq!(rebuilt.spring, base.spring);
    }
}
