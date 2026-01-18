use anyhow::Result;

use crate::ecs::context::FrameContext;
use crate::ecs::{animation_time_system, playback_update_animations, transform_propagation_system};
use crate::ecs::AnimationPlayback;

pub unsafe fn run_animation_phase(ctx: &mut FrameContext) -> Result<()> {
    {
        let mut playback = ctx.world.resource_mut::<AnimationPlayback>();
        if let Err(e) = playback_update_animations(
            ctx.graphics,
            ctx.time,
            &mut *playback,
            ctx.instance,
            ctx.device,
            ctx.command_pool.as_ref(),
            &mut ctx.raytracing.acceleration_structure,
        ) {
            eprintln!("failed to update animations: {}", e);
        }
    }

    animation_time_system(ctx.world, ctx.delta_time, ctx.assets);
    transform_propagation_system(ctx.world);

    Ok(())
}
