use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::app::FrameContext;
use crate::ecs::systems::render_data_systems::{
    gizmo_mesh_render_data, gizmo_selectable_render_data, grid_render_data,
};
use crate::ecs::{gizmo_update_rotation, gizmo_update_vertex_buffer, ProjectionData};
use crate::math::get_camera_axes_from_view;
use crate::render::RenderBackend;
use crate::renderer::scene_renderer::update_object_ubo;
use crate::vulkanr::data::UniformBufferObject;

pub unsafe fn run_render_prep_phase(ctx: &mut FrameContext) -> Result<()> {
    let (view, proj, screen_size, aspect) = {
        let proj_data = ctx.world.resource::<ProjectionData>();
        (
            proj_data.view,
            proj_data.proj,
            proj_data.screen_size,
            proj_data.aspect,
        )
    };

    let camera_position = ctx.camera().position;
    let light_position = ctx.rt_debug().light_position;

    {
        let proj_data = ProjectionData {
            view,
            proj,
            screen_size,
            aspect,
        };
        let image_index = ctx.image_index;
        let mut backend = ctx.create_backend();
        backend.update_frame_ubo(
            &proj_data,
            camera_position,
            light_position,
            Vector3::new(1.0, 1.0, 1.0),
            image_index,
        )?;
    }

    if let Err(e) = ctx
        .graphics
        .update_objects(ctx.device, ctx.image_index, Matrix4::identity())
    {
        eprintln!("Failed to update ObjectUBO: {}", e);
    }

    {
        let rt_debug = ctx.rt_debug();
        let light_pos = rt_debug.light_position;
        let debug_mode = rt_debug.debug_view_mode.as_int();
        let shadow_strength = rt_debug.shadow_strength;
        let enable_distance_attenuation = rt_debug.enable_distance_attenuation;
        drop(rt_debug);

        let mut backend = ctx.create_backend();
        backend.update_scene_uniform(
            view,
            proj,
            light_pos,
            Vector3::new(1.0, 1.0, 1.0),
            debug_mode,
            shadow_strength,
            enable_distance_attenuation,
        )?;
    }

    let render_data_vec = vec![
        grid_render_data(&ctx.grid()),
        gizmo_mesh_render_data(&ctx.gizmo()),
        gizmo_selectable_render_data(&ctx.light_gizmo(), camera_position),
    ];
    let render_data_refs: Vec<_> = render_data_vec.iter().collect();

    if let Err(e) = update_object_ubo(
        &render_data_refs,
        ctx.image_index,
        &ctx.graphics.objects,
        ctx.device,
    ) {
        eprintln!("Failed to update object UBOs: {}", e);
    }

    update_billboard_ubo(ctx, view, proj)?;

    update_grid_gizmo_buffers(ctx, view)?;

    Ok(())
}

unsafe fn update_billboard_ubo(
    ctx: &mut FrameContext,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
) -> Result<()> {
    let mut billboard = ctx.billboard_mut();

    let model_matrix = billboard
        .info
        .transform
        .as_ref()
        .map(|t| t.model_matrix)
        .unwrap_or(Matrix4::identity());

    for i in 0..billboard.render.descriptor_set.rrdata.len() {
        let rrdata = &mut billboard.render.descriptor_set.rrdata[i];

        let ubo_billboard = UniformBufferObject {
            model: model_matrix,
            view,
            proj,
        };

        let name = format!("billboard[{}]", i);
        rrdata.rruniform_buffers[ctx.image_index].update(ctx.device, &ubo_billboard, &name)?;
    }

    Ok(())
}

unsafe fn update_grid_gizmo_buffers(ctx: &mut FrameContext, view: Matrix4<f32>) -> Result<()> {
    let (camera_right, camera_up_gizmo, camera_forward) = get_camera_axes_from_view(view);

    let gizmo_rotation =
        cgmath::Matrix3::from_cols(camera_right, camera_up_gizmo, camera_forward);

    gizmo_update_rotation(&mut ctx.gizmo_mut().mesh, &gizmo_rotation);

    let mesh = ctx.gizmo().mesh.clone();
    let backend = ctx.create_backend();
    gizmo_update_vertex_buffer(&mesh, &backend)?;

    Ok(())
}
