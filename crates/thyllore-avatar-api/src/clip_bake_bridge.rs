use std::collections::HashMap;

use cgmath::{InnerSpace, Matrix, Matrix3, Quaternion};
use thyllore_anim_core::editable::{
    clip_remap_bone_ids, AnimationClipFile, EditableAnimationClip, ANIMATION_FORMAT_VERSION,
};
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::humanoid_frame::derive_humanoid_frame;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose;
use thyllore_avatar_core::motion::components::retarget_context::RetargetContext;
use thyllore_avatar_core::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};
use thyllore_avatar_core::motion::systems::bake::bake_humanoid_clip;
use thyllore_avatar_core::motion::systems::retarget_pose::build_retarget_context;
use thyllore_model_core::BoneId;

use crate::avatar_setup_bridge::BoneSpec;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct BoneRotationTrack {
    pub bone: String,
    /// Parent-relative rotation per frame as (w, x, y, z); a root bone's is its engine model-space rotation.
    pub rotations: Vec<[f32; 4]>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct BakedClip {
    pub name: String,
    pub fps: u32,
    pub frame_times: Vec<f32>,
    pub hips_bone: String,
    /// Engine model-space offset of the hips in metres, one per frame.
    pub hips_offsets: Vec<[f32; 3]>,
    pub bone_rotations: Vec<BoneRotationTrack>,
}

/// Bakes a humanoid standard-space clip file onto the rig the way the engine plays a template:
/// role-named tracks bind to the inferred humanoid bones, then every frame is retargeted.
pub fn bake_clip_file(bones: &[BoneSpec], clip_ron: &str, fps: u32) -> anyhow::Result<BakedClip> {
    let mut clip = parse_clip(clip_ron)?;
    let inputs: Vec<BoneInput> = bones.iter().cloned().map(Into::into).collect();
    let (mapping, _) = infer_mapping(&inputs);
    clip_remap_bone_ids(&mut clip, &track_name_table(&inputs, &mapping));

    let context = retarget_context(bones, &inputs, &mapping)?;
    let baked = bake_humanoid_clip(&context, &clip, fps);
    let hips_index = *context
        .mapping
        .by_role
        .get(&HumanoidRole::Hips)
        .ok_or_else(|| anyhow::anyhow!("the rig has no Hips bone"))?;

    let bone_rotations = baked
        .bone_rotations
        .iter()
        .map(|(&index, rotations)| BoneRotationTrack {
            bone: bones[index].name.clone(),
            rotations: rotations
                .iter()
                .map(|q| [q.s, q.v.x, q.v.y, q.v.z])
                .collect(),
        })
        .collect();

    Ok(BakedClip {
        name: clip.name,
        fps,
        frame_times: baked.frame_times,
        hips_bone: bones[hips_index].name.clone(),
        hips_offsets: baked.hips_offsets.iter().map(|v| [v.x, v.y, v.z]).collect(),
        bone_rotations,
    })
}

fn parse_clip(clip_ron: &str) -> anyhow::Result<EditableAnimationClip> {
    let mut file: AnimationClipFile = ron::from_str(clip_ron)?;
    if !(1..=ANIMATION_FORMAT_VERSION).contains(&file.version) {
        anyhow::bail!(
            "clip format version {} is not supported (expected 1..={})",
            file.version,
            ANIMATION_FORMAT_VERSION
        );
    }

    file.scalar_curves.clear();
    file.into_clip(|_| None).map_err(|e| anyhow::anyhow!(e))
}

fn track_name_table(bones: &[BoneInput], mapping: &HumanoidMapping) -> HashMap<String, BoneId> {
    let mut table: HashMap<String, BoneId> = bones
        .iter()
        .enumerate()
        .map(|(index, bone)| (bone.name.clone(), index as BoneId))
        .collect();
    for (role, &index) in &mapping.by_role {
        table.insert(role.unity_name().to_string(), index as BoneId);
    }
    table
}

fn retarget_context(
    bones: &[BoneSpec],
    inputs: &[BoneInput],
    mapping: &HumanoidMapping,
) -> anyhow::Result<RetargetContext> {
    let frame = derive_humanoid_frame(mapping, inputs).ok_or_else(|| {
        anyhow::anyhow!("the rig needs Hips, Head and both upper arms to derive its frame")
    })?;
    let rest_pose = detect_rest_pose(mapping, inputs);
    let skeleton = retarget_skeleton(bones)?;
    build_retarget_context(&skeleton, mapping, &frame, rest_pose)
        .ok_or_else(|| anyhow::anyhow!("the rig needs Hips and both feet to retarget a clip"))
}

fn retarget_skeleton(bones: &[BoneSpec]) -> anyhow::Result<RetargetSkeleton> {
    let bones = bones
        .iter()
        .map(|bone| {
            let rows = bone
                .rest_frame
                .ok_or_else(|| anyhow::anyhow!("bone {:?} has no rest_frame", bone.name))?;
            Ok(RetargetBone {
                parent: bone.parent,
                world_position: bone.rest_position.into(),
                world_rotation: rotation_of_rest_frame(rows),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(RetargetSkeleton { bones })
}

fn rotation_of_rest_frame(rows: [[f32; 3]; 3]) -> Quaternion<f32> {
    let frame = Matrix3::from(rows).transpose();
    Quaternion::from(Matrix3::from_cols(
        frame.x.normalize(),
        frame.y.normalize(),
        frame.z.normalize(),
    ))
}

#[cfg(test)]
mod tests;
