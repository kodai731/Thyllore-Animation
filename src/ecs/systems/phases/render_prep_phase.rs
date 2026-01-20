use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::context::FrameContext;
use crate::ecs::systems::render_data_systems::{
    gizmo_mesh_render_data, gizmo_selectable_render_data, grid_render_data,
};
use crate::ecs::{gizmo_update_rotation, gizmo_update_vertex_buffer, update_frame_ubo, ProjectionData};
use crate::math::get_camera_axes_from_view;
use crate::renderer::scene_renderer::update_object_ubo;
use crate::vulkanr::data::{SceneUniformData, UniformBufferObject};

pub unsafe fn run_render_prep_phase(ctx: &mut FrameContext) -> Result<()> {
    let (view, proj) = {
        let proj_data = ctx.world.resource::<ProjectionData>();
        (proj_data.view, proj_data.proj)
    };

    let camera_position = ctx.camera().position;
    let light_position = ctx.rt_debug().light_position;

    {
        let proj_data = ctx.world.resource::<ProjectionData>();
        update_frame_ubo(
            ctx.graphics,
            &*proj_data,
            camera_position,
            light_position,
            Vector3::new(1.0, 1.0, 1.0),
            ctx.image_index,
            ctx.device,
        )?;
    }

    if let Err(e) = ctx
        .graphics
        .update_objects(ctx.device, ctx.image_index, Matrix4::identity())
    {
        eprintln!("Failed to update ObjectUBO: {}", e);
    }

    update_scene_uniform(ctx, view, proj)?;

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

unsafe fn update_scene_uniform(
    ctx: &mut FrameContext,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
) -> Result<()> {
    let (scene_buffer, scene_memory) = match (
        ctx.raytracing.scene_uniform_buffer,
        ctx.raytracing.scene_uniform_buffer_memory,
    ) {
        (Some(b), Some(m)) => (b, m),
        _ => return Ok(()),
    };

    let rt_debug = ctx.rt_debug();
    let light_pos = &rt_debug.light_position;

    let scene_data = SceneUniformData {
        light_position: crate::math::Vec4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
        light_color: crate::math::Vec4::new(1.0, 1.0, 1.0, 1.0),
        view,
        proj,
        debug_mode: rt_debug.debug_view_mode.as_int(),
        shadow_strength: rt_debug.shadow_strength,
        enable_distance_attenuation: if rt_debug.enable_distance_attenuation {
            1
        } else {
            0
        },
        _padding: 0,
    };

    let data_ptr = ctx.device.device.map_memory(
        scene_memory,
        0,
        std::mem::size_of::<SceneUniformData>() as u64,
        vk::MemoryMapFlags::empty(),
    )?;

    std::ptr::copy_nonoverlapping(
        &scene_data as *const SceneUniformData,
        data_ptr as *mut SceneUniformData,
        1,
    );

    ctx.device.device.unmap_memory(scene_memory);

    Ok(())
}

unsafe fn update_billboard_ubo(
    ctx: &mut FrameContext,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
) -> Result<()> {
    let mut billboard = ctx.billboard_mut();

    let model_matrix = billboard
        .transform
        .as_ref()
        .map(|t| t.model_matrix)
        .unwrap_or(Matrix4::identity());

    for i in 0..billboard.descriptor_set.rrdata.len() {
        let rrdata = &mut billboard.descriptor_set.rrdata[i];

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

    gizmo_update_vertex_buffer(&ctx.gizmo().mesh, ctx.buffer_registry, ctx.device)?;

    Ok(())
}
