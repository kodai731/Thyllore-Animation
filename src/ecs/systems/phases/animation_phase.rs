use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{AnimationPlayback, AnimationRegistry, ModelState, NodeAssets};
use crate::ecs::{
    animation_time_system, playback_prepare_animations, playback_upload_animations,
    transform_propagation_system,
};

pub struct AnimationUpdates {
    pub updated_meshes: Vec<usize>,
}

pub fn run_animation_phase_ecs(ctx: &mut FrameContext) -> AnimationUpdates {
    let updated_meshes = {
        let mut playback = ctx.world.resource_mut::<AnimationPlayback>();
        let mut anim_registry = ctx.world.resource_mut::<AnimationRegistry>();
        let model_state = ctx.world.resource::<ModelState>();
        let mut node_assets = ctx.world.resource_mut::<NodeAssets>();

        playback_prepare_animations(
            ctx.graphics,
            &mut node_assets.nodes,
            &mut *anim_registry,
            &*model_state,
            ctx.time,
            &mut *playback,
        )
    };

    animation_time_system(ctx.world, ctx.delta_time, ctx.assets);
    transform_propagation_system(ctx.world);

    AnimationUpdates { updated_meshes }
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
