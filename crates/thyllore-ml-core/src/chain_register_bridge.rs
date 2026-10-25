use cgmath::{Matrix, Matrix4, SquareMatrix, Vector3};
use thyllore_anim_core::spring_bone::{
    ensure_fitted_collider, fit_bone_capsule, CapsuleFitParams, ChainOrigin, FittedCapsule,
    SpringChain, SpringColliderGroupId, SpringJointParam, FITTED_BODY_GROUP_NAME,
};
use thyllore_avatar_core::garment::components::submesh_role::SubmeshSelector;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar, SidecarSpring,
};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

use crate::chain_gen_bridge::{role_from_str, CapsuleOut};
use crate::collider_bridge::{
    skeleton_from_fit_bones, skin_data_from_fit_skin, to_bone_inputs, FitBone, FitSkin,
};

#[derive(Clone, serde::Deserialize)]
pub struct GeneratedChainIn {
    pub name: String,
    pub joints: Vec<GeneratedJointIn>,
}

#[derive(Clone, serde::Deserialize)]
pub struct GeneratedJointIn {
    pub name: String,
    pub hit_radius: f32,
}

pub struct RegisterRequest<'a> {
    pub root_name: &'a str,
    pub target: &'a str,
    pub role: &'a str,
    pub chains: &'a [GeneratedChainIn],
    pub head_capsule: Option<CapsuleOut>,
}

/// Registers generated chains the way the engine does: the chains previously generated on the
/// target are dropped, the root collider (the head capsule, or a capsule fitted to the skin on
/// the root bone) joins the fitted body group, and every chain is bound to that group.
pub fn register_generated_chains(
    spring: &SidecarSpring,
    bones: &[FitBone],
    skins: &[FitSkin],
    request: &RegisterRequest<'_>,
) -> anyhow::Result<SidecarSpring> {
    let target: SubmeshSelector = request.target.parse().map_err(anyhow::Error::msg)?;
    let role = role_from_str(request.role)?;
    let skeleton = skeleton_from_fit_bones(bones)?;
    let bone_inputs = to_bone_inputs(bones);
    let root = bone_id(&skeleton, request.root_name)?;

    let mut setup = spring_setup_from_sidecar(spring, &bone_inputs);
    setup
        .chains
        .retain(|chain| !chain.origin.is_generated_on(&target));

    let root_collider = match request.head_capsule {
        Some(capsule) => Some(head_capsule_collider(&capsule, root, bones)?),
        None => fit_root_collider(&skeleton, root, skins),
    };
    let fitted_body_group = match root_collider {
        Some(capsule) => Some(ensure_fitted_collider(&mut setup, &capsule)),
        None => fitted_body_group_id(&setup),
    };

    for generated in request.chains {
        let joints = generated
            .joints
            .iter()
            .map(|joint| {
                Ok(SpringJointParam {
                    bone_id: bone_id(&skeleton, &joint.name)?,
                    hit_radius: joint.hit_radius,
                    ..SpringJointParam::default()
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let chain = SpringChain {
            id: setup.next_chain_id,
            name: generated.name.clone(),
            joints,
            collider_group_ids: fitted_body_group.into_iter().collect(),
            origin: ChainOrigin::Generated {
                target: target.clone(),
                role,
            },
            ..SpringChain::default()
        };
        setup.next_chain_id += 1;
        setup.chains.push(chain);
    }

    Ok(build_sidecar_spring(&setup, &bone_inputs))
}

fn bone_id(skeleton: &Skeleton, name: &str) -> anyhow::Result<BoneId> {
    skeleton
        .bone_name_to_id
        .get(name)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("bone \"{}\" not found", name))
}

fn fitted_body_group_id(
    setup: &thyllore_anim_core::spring_bone::SpringBoneSetup,
) -> Option<SpringColliderGroupId> {
    setup
        .collider_groups
        .iter()
        .find(|group| group.name == FITTED_BODY_GROUP_NAME)
        .map(|group| group.id)
}

fn head_capsule_collider(
    capsule: &CapsuleOut,
    root: BoneId,
    bones: &[FitBone],
) -> anyhow::Result<FittedCapsule> {
    let root_global = Matrix4::from(bones[root as usize].bind_matrix).transpose();
    let model_to_root_local = root_global
        .invert()
        .ok_or_else(|| anyhow::anyhow!("bind matrix of the root bone is singular"))?;
    let to_local = |p: [f32; 3]| (model_to_root_local * Vector3::from(p).extend(1.0)).truncate();
    Ok(FittedCapsule {
        bone_id: root,
        head: to_local(capsule.bottom),
        tail: to_local(capsule.top),
        radius: capsule.radius,
    })
}

fn fit_root_collider(
    skeleton: &Skeleton,
    root: BoneId,
    skins: &[FitSkin],
) -> Option<FittedCapsule> {
    let owned: Vec<SkinData> = skins.iter().map(skin_data_from_fit_skin).collect();
    let skin_data: Vec<&SkinData> = owned.iter().collect();
    fit_bone_capsule(
        &skin_data,
        skeleton.get_bone(root)?,
        &CapsuleFitParams::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar_setup_bridge::BoneSpec;
    use thyllore_anim_core::spring_bone::ColliderShape;

    fn fit_bone(name: &str, parent: Option<usize>, head: [f32; 3]) -> FitBone {
        let mut bind_matrix = [[0.0; 4]; 4];
        for (i, row) in bind_matrix.iter_mut().enumerate() {
            row[i] = 1.0;
            row[3] = head[i.min(2)];
        }
        bind_matrix[3] = [0.0, 0.0, 0.0, 1.0];
        FitBone {
            bone: BoneSpec {
                name: name.to_string(),
                parent,
                rest_position: head,
            },
            bind_matrix,
        }
    }

    fn bones() -> Vec<FitBone> {
        vec![
            fit_bone("Hips", None, [0.0, 1.0, 0.0]),
            fit_bone("Head", Some(0), [0.0, 1.5, 0.0]),
            fit_bone("hair_hair_back_00_00", Some(1), [0.0, 1.6, -0.1]),
            fit_bone("hair_hair_back_00_01", Some(2), [0.0, 1.4, -0.1]),
        ]
    }

    fn request<'a>(
        chains: &'a [GeneratedChainIn],
        head_capsule: Option<CapsuleOut>,
    ) -> RegisterRequest<'a> {
        RegisterRequest {
            root_name: "Head",
            target: "Hair_back/Hair",
            role: "hair",
            chains,
            head_capsule,
        }
    }

    fn generated() -> Vec<GeneratedChainIn> {
        vec![GeneratedChainIn {
            name: "hair_hair_back_00".to_string(),
            joints: vec![
                GeneratedJointIn {
                    name: "hair_hair_back_00_00".to_string(),
                    hit_radius: 0.012,
                },
                GeneratedJointIn {
                    name: "hair_hair_back_00_01".to_string(),
                    hit_radius: 0.008,
                },
            ],
        }]
    }

    fn capsule() -> CapsuleOut {
        CapsuleOut {
            bottom: [0.0, 1.55, 0.0],
            top: [0.0, 1.7, 0.0],
            radius: 0.09,
        }
    }

    #[test]
    fn head_capsule_becomes_a_fitted_body_collider_on_the_root_in_bone_space() {
        let chains = generated();
        let spring = register_generated_chains(
            &SidecarSpring::default(),
            &bones(),
            &[],
            &request(&chains, Some(capsule())),
        )
        .unwrap();

        assert_eq!(spring.colliders.len(), 1);
        let collider = &spring.colliders[0];
        assert_eq!(collider.bone, "Head");
        let bone_inputs = to_bone_inputs(&bones());
        let setup = spring_setup_from_sidecar(&spring, &bone_inputs);
        match setup.colliders[0].shape {
            ColliderShape::Capsule { radius, tail } => {
                assert!((radius - 0.09).abs() < 1e-6);
                assert!((tail.y - 0.2).abs() < 1e-5);
            }
            ColliderShape::Sphere { .. } => panic!("the head collider is a capsule"),
        }
        assert!((setup.colliders[0].offset.y - 0.05).abs() < 1e-5);

        assert_eq!(spring.collider_groups.len(), 1);
        assert_eq!(spring.collider_groups[0].name, FITTED_BODY_GROUP_NAME);
        assert_eq!(spring.chains.len(), 1);
        assert_eq!(spring.chains[0].collider_groups, [FITTED_BODY_GROUP_NAME]);
        let radii: Vec<f32> = spring.chains[0]
            .joints
            .iter()
            .map(|j| j.hit_radius)
            .collect();
        assert_eq!(radii, [0.012, 0.008]);
    }

    #[test]
    fn regenerating_the_same_target_replaces_its_chains_and_root_collider() {
        let chains = generated();
        let first = register_generated_chains(
            &SidecarSpring::default(),
            &bones(),
            &[],
            &request(&chains, Some(capsule())),
        )
        .unwrap();
        let second =
            register_generated_chains(&first, &bones(), &[], &request(&chains, Some(capsule())))
                .unwrap();

        assert_eq!(second.chains.len(), 1);
        assert_eq!(second.colliders.len(), 1);
        assert_eq!(second.collider_groups[0].colliders.len(), 1);
    }

    #[test]
    fn without_a_capsule_or_skin_the_chains_join_an_existing_fitted_group_only() {
        let chains = generated();
        let spring = register_generated_chains(
            &SidecarSpring::default(),
            &bones(),
            &[],
            &request(&chains, None),
        )
        .unwrap();

        assert!(spring.colliders.is_empty());
        assert!(spring.collider_groups.is_empty());
        assert!(spring.chains[0].collider_groups.is_empty());
    }

    #[test]
    fn unknown_joint_bone_is_an_error() {
        let chains = vec![GeneratedChainIn {
            name: "x".to_string(),
            joints: vec![GeneratedJointIn {
                name: "missing".to_string(),
                hit_radius: 0.0,
            }],
        }];
        assert!(register_generated_chains(
            &SidecarSpring::default(),
            &bones(),
            &[],
            &request(&chains, None)
        )
        .is_err());
    }
}
