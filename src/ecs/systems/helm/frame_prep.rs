use crate::ecs::context::EcsContext;
use crate::ecs::systems::helm::batch_driver::{run_after_helm, run_before_helm};
use crate::ecs::systems::phases::run_helm_phase;
use crate::ecs::FrameContext;

fn helm_frame_prep(ctx: &mut FrameContext) {
    let mut ecs_ctx = EcsContext {
        time: ctx.time,
        delta_time: ctx.delta_time,
        image_index: ctx.image_index,
        swapchain_extent: ctx.swapchain_extent,
        world: ctx.world,
        assets: ctx.assets,
    };
    run_before_helm(&mut ecs_ctx);
    run_helm_phase(&mut ecs_ctx);
    run_after_helm(&mut ecs_ctx);
}

crate::frame_prep_hook!("helm", Advance, helm_frame_prep);
