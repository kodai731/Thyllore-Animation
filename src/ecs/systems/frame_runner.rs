use anyhow::Result;

use super::phases::{
    collect_mesh_positions, run_animation_phase_ecs, run_animation_phase_gpu, run_first_phase,
    run_input_phase, run_onion_skin_phase, run_render_prep_phase, run_timeline_phase,
    run_transform_phase_ecs, run_transform_phase_gpu,
};
use crate::ecs::context::EcsContext;
use crate::ecs::FrameContext;

pub unsafe fn run_frame(ctx: &mut FrameContext) -> Result<()> {
    let mut stages: Vec<(String, f32)> = Vec::new();

    let t = std::time::Instant::now();
    run_first_phase(ctx.world);
    stages.push((
        "run_first_phase".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    let mesh_positions = collect_mesh_positions(ctx.graphics);
    {
        let mut ecs_ctx = EcsContext {
            time: ctx.time,
            delta_time: ctx.delta_time,
            image_index: ctx.image_index,
            swapchain_extent: ctx.swapchain_extent,
            world: ctx.world,
            assets: ctx.assets,
            mesh_positions,
        };

        run_input_phase(&mut ecs_ctx)?;
        run_transform_phase_ecs(&mut ecs_ctx);
    }
    stages.push((
        "run_input_phase".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    run_timeline_phase(ctx);
    stages.push((
        "run_timeline_phase".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    let animation_updates = run_animation_phase_ecs(ctx);
    stages.push((
        "run_animation_phase_ecs".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    run_animation_phase_gpu(ctx, &animation_updates)?;
    stages.push((
        "run_animation_phase_gpu".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    run_onion_skin_phase(ctx, &animation_updates.updated_meshes)?;
    stages.push((
        "run_onion_skin_phase".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    run_transform_phase_gpu(ctx)?;
    stages.push((
        "run_transform_phase_gpu".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    let t = std::time::Instant::now();
    run_render_prep_phase(ctx)?;
    stages.push((
        "run_render_prep_phase".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    ));

    ctx.world
        .insert_resource(crate::ecs::resource::UpdatePhaseTimings { stages });
    Ok(())
}
