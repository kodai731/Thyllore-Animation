use cgmath::{Matrix, Matrix4, SquareMatrix, Vector3, Vector4};
use thyllore_anim_core::spring_bone::{
    fit_bone_capsules, replace_fitted_body_colliders, set_chain_collider_groups, CapsuleFitParams,
    SpringChainId, SpringColliderGroupId, CAPSULE_FIT_SCALAR_PARAMS, CAPSULE_FIT_UI_PARAMS,
};
use thyllore_avatar_core::humanoid::components::body_collider_roles::body_collider_bone_indices;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, rest_frames_from_globals, spring_setup_from_sidecar, BoneRestFrame,
    SidecarSpring,
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

/// The sidecar frame of each bone: the bind matrix's axes, whose lengths are already metres.
pub(crate) fn rest_frames(bones: &[FitBone]) -> anyhow::Result<Vec<BoneRestFrame>> {
    let bind_matrices: Vec<Matrix4<f32>> = bones
        .iter()
        .map(|fit_bone| Matrix4::from(fit_bone.bind_matrix).transpose())
        .collect();
    rest_frames_from_globals(&bind_matrices, &to_bone_inputs(bones), 1.0)
}

pub(crate) fn skeleton_from_fit_bones(bones: &[FitBone]) -> anyhow::Result<Skeleton> {
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

pub(crate) fn skin_data_from_fit_skin(fit_skin: &FitSkin) -> SkinData {
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

pub(crate) fn to_bone_inputs(bones: &[FitBone]) -> Vec<BoneInput> {
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

    let frames = rest_frames(bones)?;
    let mut setup = spring_setup_from_sidecar(spring, &bone_inputs, &frames);
    replace_fitted_body_colliders(&mut setup, &fitted);

    Ok(build_sidecar_spring(&setup, &bone_inputs, &frames))
}

pub fn assign_collider_groups(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    chain_names: &[String],
    group_names: &[String],
) -> anyhow::Result<SidecarSpring> {
    let mut setup =
        spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));

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

    Ok(build_sidecar_spring(
        &setup,
        bones,
        &BoneRestFrame::identity_for(bones.len()),
    ))
}

pub fn capsule_fit_params() -> Vec<ParamRow> {
    param_rows(
        &CAPSULE_FIT_UI_PARAMS,
        &CAPSULE_FIT_SCALAR_PARAMS,
        &CapsuleFitParams::default(),
    )
}

#[cfg(test)]
mod tests;
