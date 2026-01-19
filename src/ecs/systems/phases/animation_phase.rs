use anyhow::Result;

use crate::ecs::context::FrameContext;
use crate::ecs::resource::{AnimationPlayback, AnimationRegistry, ModelState, NodeAssets};
use crate::ecs::{
    animation_time_system, playback_prepare_animations, playback_upload_animations,
    transform_propagation_system,
};

pub unsafe fn run_animation_phase(ctx: &mut FrameContext) -> Result<()> {
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

    if !updated_meshes.is_empty() {
        let mut backend = ctx.create_backend();
        if let Err(e) = playback_upload_animations(&mut backend, &updated_meshes) {
            eprintln!("failed to upload animations: {}", e);
        }
    }

    animation_time_system(ctx.world, ctx.delta_time, ctx.assets);
    transform_propagation_system(ctx.world);

    Ok(())
}
