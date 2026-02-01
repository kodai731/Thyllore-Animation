use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::app::FrameContext;
use crate::debugview::gizmo::{BoneDisplayStyle, BoneGizmoData};
use crate::ecs::systems::render_data_systems::{
    bone_gizmo_render_data, gizmo_mesh_render_data, gizmo_selectable_render_data,
    grid_mesh_render_data,
};
use crate::ecs::component::LineMesh;
use crate::debugview::gizmo::BoneSelectionState;
use crate::ecs::{
    build_bone_line_mesh, build_octahedral_bone_meshes_with_selection,
    gizmo_update_rotation, gizmo_update_vertex_buffer, ProjectionData,
};
use crate::math::get_camera_axes_from_view;
use crate::render::RenderBackend;
use crate::renderer::scene_renderer::update_object_ubo;

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

    let mut render_data_vec = vec![
        grid_mesh_render_data(&ctx.grid_mesh()),
        gizmo_mesh_render_data(&ctx.gizmo()),
        gizmo_selectable_render_data(&ctx.light_gizmo(), camera_position),
    ];

    if ctx.world.contains_resource::<BoneGizmoData>() {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        if bone_gizmo.visible {
            render_data_vec.extend(bone_gizmo_render_data(&bone_gizmo));
        }
    }

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
    update_bone_gizmo_mesh(ctx)?;

    Ok(())
}

unsafe fn update_billboard_ubo(
    ctx: &mut FrameContext,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
) -> Result<()> {
    let model_matrix = {
        let billboard = ctx.billboard();
        billboard
            .transform
            .as_ref()
            .map(|t| t.model_matrix)
            .unwrap_or(Matrix4::identity())
    };

    let image_index = ctx.image_index;
    ctx.update_billboard_ubo_internal(model_matrix, view, proj, image_index)?;

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

unsafe fn update_bone_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let (visible, display_style, skeleton_id, transforms, offsets) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.visible,
            bone_gizmo.display_style,
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
        )
    };

    if !visible {
        return Ok(());
    }

    let Some(skel_id) = skeleton_id else {
        return Ok(());
    };

    let Some(skeleton) = ctx.assets.get_skeleton_by_skeleton_id(skel_id) else {
        return Ok(());
    };
    let skeleton = skeleton.clone();

    match display_style {
        BoneDisplayStyle::Stick => {
            update_stick_bone_mesh(ctx, &skeleton, &transforms, &offsets)?;
        }
        BoneDisplayStyle::Octahedral => {
            update_octahedral_bone_mesh(ctx, &skeleton, &transforms, &offsets)?;
        }
    }

    Ok(())
}

unsafe fn update_stick_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
) -> Result<()> {
    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        build_bone_line_mesh(skeleton, transforms, offsets, &mut bone_gizmo.stick_mesh);
    }

    let mut mesh_clone = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        bone_gizmo.stick_mesh.clone()
    };

    {
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut mesh_clone)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.stick_mesh.vertex_buffer_handle = mesh_clone.vertex_buffer_handle;
        bone_gizmo.stick_mesh.index_buffer_handle = mesh_clone.index_buffer_handle;
    }

    Ok(())
}

unsafe fn update_octahedral_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
) -> Result<()> {
    let selection = ctx
        .world
        .get_resource::<BoneSelectionState>()
        .map(|s| (*s).clone())
        .unwrap_or_default();

    let mut solid_mesh = LineMesh::default();
    let mut wire_mesh = LineMesh::default();
    build_octahedral_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh)?;
        backend.update_or_create_line_buffers(&mut wire_mesh)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}
