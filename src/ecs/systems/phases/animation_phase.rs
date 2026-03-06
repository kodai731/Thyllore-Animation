use std::collections::HashMap;

use anyhow::Result;

use crate::animation::{BoneId, BoneLocalPose};
use crate::app::FrameContext;
use crate::debugview::gizmo::BoneGizmoData;
use crate::ecs::resource::{BonePoseOverride, ClipLibrary, NodeAssets};
use crate::ecs::{
    evaluate_all_animators, playback_upload_animations, transform_propagation_system,
};

pub struct AnimationUpdates {
    pub updated_meshes: Vec<usize>,
}

pub fn run_animation_phase_ecs(ctx: &mut FrameContext) -> AnimationUpdates {
    let pose_overrides: HashMap<BoneId, BoneLocalPose> = ctx
        .world
        .get_resource::<BonePoseOverride>()
        .map(|r| r.overrides.clone())
        .unwrap_or_default();

    let eval_result = {
        let clip_library = ctx.world.resource::<ClipLibrary>();
        let mut node_assets = ctx.world.resource_mut::<NodeAssets>();

        evaluate_all_animators(
            ctx.world,
            ctx.graphics,
            &mut node_assets.nodes,
            &*clip_library,
            ctx.assets,
            ctx.delta_time,
            &pose_overrides,
        )
    };

    if let Some((skel_id, transforms)) = &eval_result.bone_transforms {
        if ctx.world.contains_resource::<BoneGizmoData>() {
            let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
            bone_gizmo.cached_skeleton_id = Some(*skel_id);
            bone_gizmo.cached_global_transforms = transforms.clone();
        }
    }

    transform_propagation_system(ctx.world);

    AnimationUpdates {
        updated_meshes: eval_result.updated_meshes,
    }
}

pub unsafe fn run_animation_phase_gpu(
    ctx: &mut FrameContext,
    updates: &AnimationUpdates,
) -> Result<()> {
    if !updates.updated_meshes.is_empty() {
        let mut backend = ctx.create_backend();
        playback_upload_animations(&mut backend, &updates.updated_meshes)?;
    }

    Ok(())
}
