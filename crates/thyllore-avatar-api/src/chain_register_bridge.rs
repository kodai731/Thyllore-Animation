use glam::{Mat4, Vec3};
use thyllore_anim_core::spring_bone::{
    add_generated_chains, fit_bone_capsule, CapsuleFitParams, ChainOrigin, FittedCapsule,
    RootAttachment, SpringChain, SpringJointParam, SpringSpace,
};
use thyllore_avatar_core::cloth_setup::components::submesh_role::SubmeshSelector;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar, SidecarSpring,
};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

use crate::chain_gen_bridge::CapsuleOut;
use crate::collider_bridge::{
    rest_frames, skeleton_from_fit_bones, skin_data_from_fit_skin, to_bone_inputs, FitBone, FitSkin,
};

#[derive(Clone, serde::Deserialize)]
pub struct GeneratedChainIn {
    pub name: String,
    pub joints: Vec<GeneratedJointIn>,
    /// The bone at the chain's end point, the last joint's tail
    pub end_bone: String,
}

#[derive(Clone, serde::Deserialize)]
pub struct GeneratedJointIn {
    pub name: String,
    pub hit_radius: f32,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterGeneratedChainsRequest {
    pub spring: SidecarSpring,
    pub bones: Vec<FitBone>,
    pub skins: Vec<FitSkin>,
    pub root_bone: String,
    pub target: String,
    pub attachment: RootAttachment,
    pub chains: Vec<GeneratedChainIn>,
    #[serde(default)]
    pub head_capsule: Option<CapsuleOut>,
}

/// Registers generated chains the way the engine does: the chains of the request replace the chains of
/// the same names previously generated on the target (the other chains of the target keep their
/// values), the root collider (the head capsule, or a capsule fitted to the skin on the root bone)
/// joins the fitted body group, and every registered chain is bound to that group.
pub fn register_generated_chains(
    request: &RegisterGeneratedChainsRequest,
) -> anyhow::Result<SidecarSpring> {
    let target: SubmeshSelector = request.target.parse().map_err(anyhow::Error::msg)?;
    let attachment = request.attachment;
    let skeleton = skeleton_from_fit_bones(&request.bones)?;
    let bone_inputs = to_bone_inputs(&request.bones);
    let root = bone_id(&skeleton, &request.root_bone)?;

    let frames = rest_frames(&request.bones)?;
    let mut setup = spring_setup_from_sidecar(&request.spring, &bone_inputs, &frames);
    let replaced: Vec<&str> = request
        .chains
        .iter()
        .map(|chain| chain.name.as_str())
        .collect();
    setup.chains.retain(|chain| {
        !(chain.origin.is_generated_on(&target) && replaced.contains(&chain.name.as_str()))
    });

    let root_collider = match &request.head_capsule {
        Some(capsule) => Some(head_capsule_collider(capsule, root, &request.bones)?),
        None => fit_root_collider(&skeleton, root, &request.skins),
    };
    let chains: Vec<SpringChain> = request
        .chains
        .iter()
        .map(|generated| {
            let joints = generated
                .joints
                .iter()
                .map(|joint| {
                    Ok(SpringJointParam {
                        bone_id: bone_id(&skeleton, &joint.name)?,
                        hit_radius: joint.hit_radius,
                        ..setup.new_joint_template()
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(SpringChain {
                name: generated.name.clone(),
                joints,
                tail_bone_id: Some(bone_id(&skeleton, &generated.end_bone)?),
                origin: ChainOrigin::Generated {
                    target: target.clone(),
                    attachment,
                },
                ..SpringChain::default()
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    add_generated_chains(&mut setup, chains, root_collider, SpringSpace::IDENTITY);

    Ok(build_sidecar_spring(&setup, &bone_inputs, &frames))
}

fn bone_id(skeleton: &Skeleton, name: &str) -> anyhow::Result<BoneId> {
    skeleton
        .bone_name_to_id
        .get(name)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("bone \"{}\" not found", name))
}

fn head_capsule_collider(
    capsule: &CapsuleOut,
    root: BoneId,
    bones: &[FitBone],
) -> anyhow::Result<FittedCapsule> {
    let root_global = Mat4::from_cols_array_2d(&bones[root as usize].bind_matrix).transpose();
    let model_to_root_local = root_global
        .try_inverse()
        .ok_or_else(|| anyhow::anyhow!("bind matrix of the root bone is singular"))?;
    let to_local = |p: [f32; 3]| (model_to_root_local * Vec3::from(p).extend(1.0)).truncate();
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
