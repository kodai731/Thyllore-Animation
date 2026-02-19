use std::collections::HashMap;

use cgmath::{Quaternion, Vector3};

use crate::animation::editable::EditableAnimationClip;
use crate::animation::{AnimationClip, BoneId, Keyframe, Skeleton, SkeletonPose, TransformChannel};
use crate::ecs::component::ConstraintSet;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::systems::{apply_constraints, create_pose_from_rest, sample_clip_to_pose};

use super::clip_library_systems::clip_library_register_and_activate;

pub fn constraint_bake_evaluate(
    source_clip: &AnimationClip,
    skeleton: &Skeleton,
    constraint_set: &ConstraintSet,
    sample_fps: f32,
    looping: bool,
) -> EditableAnimationClip {
    let duration = source_clip.duration;
    let frame_count = (duration * sample_fps).ceil() as usize + 1;
    let bone_names = collect_bone_names(skeleton);

    let mut bone_translations: Vec<Vec<(f32, Vector3<f32>)>> =
        vec![Vec::new(); skeleton.bone_count()];
    let mut bone_rotations: Vec<Vec<(f32, Quaternion<f32>)>> =
        vec![Vec::new(); skeleton.bone_count()];
    let mut bone_scales: Vec<Vec<(f32, Vector3<f32>)>> = vec![Vec::new(); skeleton.bone_count()];

    for i in 0..frame_count {
        let time = (i as f32 / sample_fps).min(duration);

        let mut pose = create_pose_from_rest(skeleton);
        sample_clip_to_pose(source_clip, time, skeleton, &mut pose, looping);
        apply_constraints(constraint_set, skeleton, &mut pose);

        record_pose_keyframes(
            &pose,
            time,
            &mut bone_translations,
            &mut bone_rotations,
            &mut bone_scales,
        );
    }

    let baked_clip = build_animation_clip(
        &format!("{}_baked", source_clip.name),
        duration,
        &bone_translations,
        &bone_rotations,
        &bone_scales,
    );

    EditableAnimationClip::from_animation_clip(0, &baked_clip, &bone_names)
}

pub fn constraint_bake_rest_pose(
    skeleton: &Skeleton,
    constraint_set: &ConstraintSet,
) -> EditableAnimationClip {
    let bone_names = collect_bone_names(skeleton);

    let mut pose = create_pose_from_rest(skeleton);
    apply_constraints(constraint_set, skeleton, &mut pose);

    let mut bone_translations: Vec<Vec<(f32, Vector3<f32>)>> =
        vec![Vec::new(); skeleton.bone_count()];
    let mut bone_rotations: Vec<Vec<(f32, Quaternion<f32>)>> =
        vec![Vec::new(); skeleton.bone_count()];
    let mut bone_scales: Vec<Vec<(f32, Vector3<f32>)>> = vec![Vec::new(); skeleton.bone_count()];

    record_pose_keyframes(
        &pose,
        0.0,
        &mut bone_translations,
        &mut bone_rotations,
        &mut bone_scales,
    );

    let baked_clip = build_animation_clip(
        "rest_baked",
        0.0,
        &bone_translations,
        &bone_rotations,
        &bone_scales,
    );

    EditableAnimationClip::from_animation_clip(0, &baked_clip, &bone_names)
}

pub fn constraint_bake_register(
    clip_library: &mut ClipLibrary,
    assets: &mut crate::asset::AssetStorage,
    baked_clip: EditableAnimationClip,
) -> crate::animation::editable::SourceClipId {
    clip_library_register_and_activate(clip_library, assets, baked_clip)
}

fn collect_bone_names(skeleton: &Skeleton) -> HashMap<BoneId, String> {
    skeleton
        .bones
        .iter()
        .map(|bone| (bone.id, bone.name.clone()))
        .collect()
}

fn record_pose_keyframes(
    pose: &SkeletonPose,
    time: f32,
    bone_translations: &mut [Vec<(f32, Vector3<f32>)>],
    bone_rotations: &mut [Vec<(f32, Quaternion<f32>)>],
    bone_scales: &mut [Vec<(f32, Vector3<f32>)>],
) {
    for (idx, bp) in pose.bone_poses.iter().enumerate() {
        bone_translations[idx].push((time, bp.translation));
        bone_rotations[idx].push((time, bp.rotation));
        bone_scales[idx].push((time, bp.scale));
    }
}

fn build_animation_clip(
    name: &str,
    duration: f32,
    bone_translations: &[Vec<(f32, Vector3<f32>)>],
    bone_rotations: &[Vec<(f32, Quaternion<f32>)>],
    bone_scales: &[Vec<(f32, Vector3<f32>)>],
) -> AnimationClip {
    let mut clip = AnimationClip::new(name);
    clip.duration = duration;

    for bone_idx in 0..bone_translations.len() {
        let translations = &bone_translations[bone_idx];
        let rotations = &bone_rotations[bone_idx];
        let scales = &bone_scales[bone_idx];

        if translations.is_empty() {
            continue;
        }

        let channel = TransformChannel {
            translation: translations
                .iter()
                .map(|(t, v)| Keyframe::new(*t, *v))
                .collect(),
            rotation: rotations
                .iter()
                .map(|(t, q)| Keyframe::new(*t, *q))
                .collect(),
            scale: scales
                .iter()
                .map(|(t, v)| Keyframe::new(*t, *v))
                .collect(),
        };

        clip.add_channel(bone_idx as BoneId, channel);
    }

    clip
}
