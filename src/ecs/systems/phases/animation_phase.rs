use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use cgmath::Matrix4;

use crate::animation::{BoneId, BoneLocalPose, SkeletonId};
use crate::ecs::resource::gizmo::BoneSelectionState;
use crate::ecs::resource::{
    AnimationType, BonePoseOverride, ClipLibrary, NodeAssets, PhaseSubTimings, PoseApplyCache,
    WeightHeatmapState,
};
use crate::ecs::FrameContext;
use crate::ecs::{
    apply_morph_weights, evaluate_morph_tracks, playback_upload_animations, run_animation_pipeline,
    sync_avatar_setup, sync_expression_library, sync_humanoid_rig, sync_material_textures,
    update_weight_heatmap,
};

pub struct AnimationUpdates {
    pub updated_meshes: Vec<usize>,
    pub bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
}

pub fn run_animation_phase_ecs(ctx: &mut FrameContext) -> AnimationUpdates {
    let mut sub: HashMap<String, f32> = HashMap::new();

    let pose_overrides: HashMap<BoneId, BoneLocalPose> = ctx
        .world
        .get_resource::<BonePoseOverride>()
        .map(|r| r.overrides.clone())
        .unwrap_or_default();

    // Initialize PoseApplyCache before taking other mutable borrows
    if !ctx.world.contains_resource::<PoseApplyCache>() {
        ctx.world.insert_resource(PoseApplyCache::default());
    }

    let t = Instant::now();
    sync_expression_library(ctx.world);
    sub.insert(
        "sync_expression_library".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    sync_avatar_setup(ctx.world, ctx.assets, ctx.graphics);
    sub.insert(
        "sync_avatar_setup".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    sync_humanoid_rig(ctx.world, ctx.assets);
    sub.insert(
        "sync_humanoid_rig".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    sync_material_textures(ctx.world, ctx.graphics);
    sub.insert(
        "sync_material_textures".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    evaluate_morph_tracks(ctx.world, ctx.assets, ctx.graphics);
    sub.insert(
        "evaluate_morph_tracks".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    let morph_updated_meshes = apply_morph_weights(ctx.world, ctx.assets, ctx.graphics);
    sub.insert(
        "apply_morph_weights".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );
    {
        let mut pose_apply_cache = ctx.world.resource_mut::<PoseApplyCache>();
        for mesh_index in &morph_updated_meshes {
            pose_apply_cache.skinned_cache.remove(mesh_index);
            pose_apply_cache.node_cache.remove(mesh_index);
        }
    }

    let t = Instant::now();
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
            &mut sub,
        )
    };
    sub.insert(
        "run_animation_pipeline".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    let heatmap_updated_meshes = apply_weight_heatmap_update(ctx);
    sub.insert(
        "apply_weight_heatmap_update".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let mut updated_meshes = eval_result.updated_meshes;
    for mesh_index in morph_updated_meshes {
        if !updated_meshes.contains(&mesh_index) {
            updated_meshes.push(mesh_index);
        }
    }
    for mesh_index in heatmap_updated_meshes {
        if !updated_meshes.contains(&mesh_index) {
            updated_meshes.push(mesh_index);
        }
    }

    ctx.world
        .resource_mut::<PhaseSubTimings>()
        .phases
        .insert("animation", sub);

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
    let t = Instant::now();
    if !updates.updated_meshes.is_empty() {
        let mut backend = ctx.create_backend();
        playback_upload_animations(&mut backend, &updates.updated_meshes)?;
    }
    ctx.world
        .resource_mut::<PhaseSubTimings>()
        .phases
        .entry("animation")
        .or_default()
        .insert("gpu_upload".to_string(), t.elapsed().as_secs_f32() * 1000.0);

    Ok(())
}
