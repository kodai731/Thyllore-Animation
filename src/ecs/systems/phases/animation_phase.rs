use std::collections::HashMap;

use anyhow::Result;
use cgmath::Matrix4;

use crate::animation::{BoneId, BoneLocalPose, SkeletonId};
use crate::ecs::resource::gizmo::BoneSelectionState;
use crate::ecs::resource::{
    AnimationType, BonePoseOverride, ClipLibrary, NodeAssets, PoseApplyCache, WeightHeatmapState,
};
use crate::ecs::FrameContext;
use crate::ecs::{playback_upload_animations, run_animation_pipeline, update_weight_heatmap};

pub struct AnimationUpdates {
    pub updated_meshes: Vec<usize>,
    pub bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
}

pub fn run_animation_phase_ecs(ctx: &mut FrameContext) -> AnimationUpdates {
    let pose_overrides: HashMap<BoneId, BoneLocalPose> = ctx
        .world
        .get_resource::<BonePoseOverride>()
        .map(|r| r.overrides.clone())
        .unwrap_or_default();

    // Initialize PoseApplyCache before taking other mutable borrows
    if !ctx.world.contains_resource::<PoseApplyCache>() {
        ctx.world.insert_resource(PoseApplyCache::default());
    }

    let eval_result = {
        let clip_library = ctx.world.resource::<ClipLibrary>();
        let mut pose_apply_cache = ctx.world.resource_mut::<PoseApplyCache>();

        let mut node_assets = ctx.world.resource_mut::<NodeAssets>();

        run_animation_pipeline(
            ctx.world,
            ctx.graphics,
            &mut node_assets.nodes,
            &*clip_library,
            ctx.assets,
            ctx.delta_time,
            &pose_overrides,
            &mut pose_apply_cache,
        )
    };

    let heatmap_updated_meshes = apply_weight_heatmap_update(ctx);

    let mut updated_meshes = eval_result.updated_meshes;
    for mesh_index in heatmap_updated_meshes {
        if !updated_meshes.contains(&mesh_index) {
            updated_meshes.push(mesh_index);
        }
    }

    AnimationUpdates {
        updated_meshes,
        bone_transforms: eval_result.bone_transforms,
    }
}

fn apply_weight_heatmap_update(ctx: &mut FrameContext) -> Vec<usize> {
    let selection = match ctx.world.get_resource::<BoneSelectionState>() {
        Some(state) => state.clone(),
        None => return Vec::new(),
    };

    let Some(mut heatmap) = ctx.world.get_resource_mut::<WeightHeatmapState>() else {
        return Vec::new();
    };

    update_weight_heatmap(ctx.graphics, &mut heatmap, &selection)
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
