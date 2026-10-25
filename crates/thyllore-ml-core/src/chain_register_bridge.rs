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
mod tests;
