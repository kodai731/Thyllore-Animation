use cgmath::{Matrix, Matrix4, SquareMatrix, Vector3, Vector4};
use thyllore_anim_core::spring_bone::{
    fit_bone_capsules, replace_fitted_body_colliders, set_chain_collider_groups, CapsuleFitParams,
    SpringChainId, SpringColliderGroupId, CAPSULE_FIT_SCALAR_PARAMS, CAPSULE_FIT_UI_PARAMS,
};
use thyllore_avatar_core::humanoid::components::body_collider_roles::body_collider_bone_indices;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar, SidecarSpring,
};
use thyllore_model_core::{Skeleton, SkinData};

use crate::avatar_setup_bridge::BoneSpec;
use crate::param_rows::{apply_scalar_pairs, param_rows, ParamRow};
use crate::spring_edit_bridge::skeleton_from_bones;

#[derive(Clone, serde::Deserialize)]
pub struct FitBone {
    #[serde(flatten)]
    pub bone: BoneSpec,
    pub bind_matrix: [[f32; 4]; 4],
}

#[derive(Clone, serde::Deserialize)]
pub struct FitSkin {
    pub positions: Vec<[f32; 3]>,
    pub influences: Vec<Vec<(usize, f32)>>,
}

fn skeleton_from_fit_bones(bones: &[FitBone]) -> anyhow::Result<Skeleton> {
    let mut skeleton = skeleton_from_bones(&to_bone_inputs(bones));

    for (skeleton_bone, fit_bone) in skeleton.bones.iter_mut().zip(bones) {
        let bind_matrix = Matrix4::from(fit_bone.bind_matrix).transpose();
        skeleton_bone.inverse_bind_pose = bind_matrix.invert().ok_or_else(|| {
            anyhow::anyhow!(
                "bind matrix for bone \"{}\" is not invertible",
                fit_bone.bone.name
            )
        })?;
    }

    Ok(skeleton)
}

fn skin_data_from_fit_skin(fit_skin: &FitSkin) -> SkinData {
    let count = fit_skin.positions.len();
    let mut bone_indices = Vec::with_capacity(count);
    let mut bone_weights = Vec::with_capacity(count);

    for influences in &fit_skin.influences {
        let mut indices = [0u32; 4];
        let mut weights = [0.0f32; 4];
        for (slot, (bone_index, weight)) in influences.iter().take(4).enumerate() {
            indices[slot] = *bone_index as u32;
            weights[slot] = *weight;
        }
        bone_indices.push(Vector4::new(indices[0], indices[1], indices[2], indices[3]));
        bone_weights.push(Vector4::new(weights[0], weights[1], weights[2], weights[3]));
    }

    let base_positions: Vec<Vector3<f32>> = fit_skin
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();

    let base_normals = vec![Vector3::new(0.0, 0.0, 0.0); count];

    SkinData {
        skeleton_id: 0,
        bone_indices,
        bone_weights,
        base_positions,
        base_normals,
    }
}

fn to_bone_inputs(bones: &[FitBone]) -> Vec<BoneInput> {
    bones
        .iter()
        .map(|fit_bone| fit_bone.bone.clone().into())
        .collect()
}

pub fn fit_body_colliders(
    spring: &SidecarSpring,
    bones: &[FitBone],
    skins: &[FitSkin],
    settings: &[(String, f32)],
) -> anyhow::Result<SidecarSpring> {
    let mut params = CapsuleFitParams::default();
    apply_scalar_pairs(&mut params, &CAPSULE_FIT_SCALAR_PARAMS, settings)?;

    let skeleton = skeleton_from_fit_bones(bones)?;
    let bone_inputs = to_bone_inputs(bones);

    let owned: Vec<SkinData> = skins.iter().map(skin_data_from_fit_skin).collect();
    let skin_data: Vec<&SkinData> = owned.iter().collect();

    let fitted = fit_bone_capsules(&skin_data, &skeleton, &params);

    let (mapping, _) = infer_mapping(&bone_inputs);
    let allowed = body_collider_bone_indices(&mapping);
    let fitted: Vec<_> = fitted
        .into_iter()
        .filter(|c| allowed.contains(&(c.bone_id as usize)))
        .collect();

    anyhow::ensure!(!fitted.is_empty(), "fit produced no body colliders");

    let mut setup = spring_setup_from_sidecar(spring, &bone_inputs);
    replace_fitted_body_colliders(&mut setup, &fitted);

    Ok(build_sidecar_spring(&setup, &bone_inputs))
}

pub fn assign_collider_groups(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    chain_names: &[String],
    group_names: &[String],
) -> anyhow::Result<SidecarSpring> {
    let mut setup = spring_setup_from_sidecar(spring, bones);

    let chain_ids: Vec<SpringChainId> = chain_names
        .iter()
        .map(|name| {
            setup
                .chains
                .iter()
                .find(|c| c.name == *name)
                .map(|c| c.id)
                .ok_or_else(|| anyhow::anyhow!("chain \"{}\" not found", name))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let group_ids: Vec<SpringColliderGroupId> = group_names
        .iter()
        .map(|name| {
            setup
                .collider_groups
                .iter()
                .find(|g| g.name == *name)
                .map(|g| g.id)
                .ok_or_else(|| anyhow::anyhow!("collider group \"{}\" not found", name))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    set_chain_collider_groups(&mut setup, &chain_ids, &group_ids);

    Ok(build_sidecar_spring(&setup, bones))
}

pub fn capsule_fit_params() -> Vec<ParamRow> {
    param_rows(
        &CAPSULE_FIT_UI_PARAMS,
        &CAPSULE_FIT_SCALAR_PARAMS,
        &CapsuleFitParams::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
                },
                bind_matrix: identity,
            },
            FitBone {
                bone: BoneSpec {
                    name: "Spine".into(),
                    parent: Some(0),
                    rest_position: [0.0, 1.1, 0.0],
                },
                bind_matrix: identity,
            },
            FitBone {
                bone: BoneSpec {
                    name: "Chest".into(),
                    parent: Some(1),
                    rest_position: [0.0, 1.2, 0.0],
                },
                bind_matrix: identity,
            },
            FitBone {
                bone: BoneSpec {
                    name: "UpperChest".into(),
                    parent: Some(2),
                    rest_position: [0.0, 1.25, 0.0],
                },
                bind_matrix: identity,
            },
            FitBone {
                bone: BoneSpec {
                    name: "Neck".into(),
                    parent: Some(3),
                    rest_position: [0.0, 1.3, 0.0],
                },
                bind_matrix: identity,
            },
            FitBone {
                bone: BoneSpec {
                    name: "Head".into(),
                    parent: Some(4),
                    rest_position: [0.0, 1.4, 0.0],
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
                        stiffness: 0.5,
                        drag_force: 0.0,
                        gravity_power: 1.0,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    },
                ],
                tail: None,
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
                        stiffness: 0.5,
                        drag_force: 0.0,
                        gravity_power: 1.0,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    },
                ],
                tail: None,
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
                        stiffness: 0.5,
                        drag_force: 0.0,
                        gravity_power: 1.0,
                        gravity_dir: [0.0, -1.0, 0.0],
                        hit_radius: 0.0,
                    },
                ],
                tail: None,
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
}
