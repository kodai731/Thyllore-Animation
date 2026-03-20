use crate::animation::editable::BlendMode;
use crate::animation::SkeletonPose;
use crate::asset::AssetStorage;
use crate::ecs::{
    blend_poses_override, compute_crossfade_factor, create_pose_from_rest, sample_clip_to_pose,
};

use super::AnimatedEntityInfo;
use crate::ecs::systems::pose_blend_systems::blend_poses_additive;

pub(crate) fn evaluate_entity_blend(
    info: &AnimatedEntityInfo,
    assets: &AssetStorage,
) -> Option<SkeletonPose> {
    let skeleton = assets.get_skeleton_by_skeleton_id(info.skeleton_id)?;
    let rest_pose = create_pose_from_rest(skeleton);

    let first_override = info
        .active_instances
        .iter()
        .find(|i| i.blend_mode == BlendMode::Override)?;

    let clip_asset = assets.animation_clips.get(&first_override.asset_id)?;

    let mut pose = rest_pose.clone();
    sample_clip_to_pose(
        &clip_asset.clip,
        first_override.local_time,
        skeleton,
        &mut pose,
        info.looping,
    );

    if first_override.weight < 1.0 {
        pose = blend_poses_override(&rest_pose, &pose, first_override.weight);
    }

    let mut prev_override = first_override;

    for inst in &info.active_instances {
        if std::ptr::eq(inst, first_override) {
            continue;
        }

        let Some(clip_asset) = assets.animation_clips.get(&inst.asset_id) else {
            continue;
        };

        match inst.blend_mode {
            BlendMode::Override => {
                let mut overlay = rest_pose.clone();
                sample_clip_to_pose(
                    &clip_asset.clip,
                    inst.local_time,
                    skeleton,
                    &mut overlay,
                    info.looping,
                );

                let crossfade = compute_crossfade_factor(
                    first_override.local_time + first_override.start_time,
                    prev_override.end_time,
                    inst.start_time,
                    prev_override.ease_out,
                    inst.blend_mode.default_ease_in(),
                );

                let blend_weight = crossfade * inst.weight;
                pose = blend_poses_override(&pose, &overlay, blend_weight);
                prev_override = inst;
            }
            BlendMode::Additive => {
                let mut additive_pose = rest_pose.clone();
                sample_clip_to_pose(
                    &clip_asset.clip,
                    inst.local_time,
                    skeleton,
                    &mut additive_pose,
                    info.looping,
                );

                pose = blend_poses_additive(&pose, &additive_pose, &rest_pose, inst.weight);
            }
        }
    }

    Some(pose)
}
