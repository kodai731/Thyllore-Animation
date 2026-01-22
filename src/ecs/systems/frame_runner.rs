use anyhow::Result;
use cgmath::Vector3;

use crate::app::FrameContext;
use crate::ecs::context::EcsContext;
use crate::app::graphics_resource::GraphicsResources;

use super::phases::{
    run_animation_phase_ecs, run_animation_phase_gpu, run_input_phase, run_render_prep_phase,
    run_transform_phase_ecs, run_transform_phase_gpu,
};

pub unsafe fn run_frame(ctx: &mut FrameContext) -> Result<()> {
    let mesh_positions = collect_mesh_positions(ctx.graphics);

    {
        let mut ecs_ctx = EcsContext {
            time: ctx.time,
            delta_time: ctx.delta_time,
            image_index: ctx.image_index,
            swapchain_extent: ctx.swapchain_extent,
            world: ctx.world,
            assets: ctx.assets,
            gui_data: ctx.gui_data,
            mesh_positions,
        };
        run_input_phase(&mut ecs_ctx)?;
        run_transform_phase_ecs(&mut ecs_ctx);
    }

    let animation_updates = run_animation_phase_ecs(ctx);
    run_animation_phase_gpu(ctx, &animation_updates)?;

    run_transform_phase_gpu(ctx)?;
    run_render_prep_phase(ctx)?;
    Ok(())
}

fn collect_mesh_positions(graphics: &GraphicsResources) -> Vec<Vector3<f32>> {
    if graphics.meshes.is_empty() {
        return Vec::new();
    }

    graphics
        .meshes
        .iter()
        .flat_map(|mesh| {
            mesh.vertex_data
                .vertices
                .iter()
                .map(|v| Vector3::new(v.pos.x, v.pos.y, v.pos.z))
        })
        .collect()
}
