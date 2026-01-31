use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{ClipLibrary, NodeAssets};
use crate::ecs::{
    evaluate_all_animators, playback_upload_animations,
    transform_propagation_system,
};

pub struct AnimationUpdates {
    pub updated_meshes: Vec<usize>,
}

pub fn run_animation_phase_ecs(ctx: &mut FrameContext) -> AnimationUpdates {
    let updated_meshes = {
        let clip_library = ctx.world.resource::<ClipLibrary>();
        let mut node_assets = ctx.world.resource_mut::<NodeAssets>();

        evaluate_all_animators(
            ctx.world,
            ctx.graphics,
            &mut node_assets.nodes,
            &*clip_library,
            ctx.assets,
        )
    };

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
