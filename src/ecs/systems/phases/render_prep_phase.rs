use crate::ecs::resource::gizmo::TransformGizmoData;
use crate::ecs::resource::gizmo::{BoneGizmoData, ConstraintGizmoData, SpringBoneGizmoData};
use crate::ecs::resource::ProjectionData;
use crate::ecs::resource::{Exposure, SceneBrightness, TransformGizmoState};
use crate::ecs::systems::render_data_systems::{
    bone_gizmo_render_data, constraint_gizmo_render_data, gizmo_mesh_render_data,
    gizmo_selectable_render_data, grid_mesh_render_data, spring_bone_gizmo_render_data,
    transform_gizmo_render_data,
};
use crate::ecs::FrameContext;
use crate::hooks::frame_prep::{FramePrepHooks, FramePrepStage};
use crate::render::RenderBackend;
use crate::vulkanr::renderer::scene_renderer::update_object_ubo;
use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector3};
use std::collections::HashMap;
use std::time::Instant;

mod bone_gizmo_mesh;
mod gizmo_meshes;
mod gpu_timings;

use bone_gizmo_mesh::update_bone_gizmo_mesh;
use gizmo_meshes::{
    update_constraint_gizmo_mesh, update_spring_bone_gizmo_mesh, update_transform_gizmo_mesh,
};
use gpu_timings::gpu_timings_write;

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

    let camera_position = {
        use crate::ecs::systems::camera_systems::compute_camera_position;
        compute_camera_position(&ctx.camera())
    };

    let mut sub: HashMap<String, f32> = HashMap::new();

    let t = Instant::now();
    update_frame_and_scene_uniforms(ctx, view, proj, screen_size, aspect, camera_position)?;
    sub.insert("uniforms".to_string(), t.elapsed().as_secs_f32() * 1000.0);

    let t = Instant::now();
    crate::ecs::systems::batch_run_update_orbit(&mut ctx.world);
    sub.insert("orbit".to_string(), t.elapsed().as_secs_f32() * 1000.0);

    run_frame_prep_hooks(ctx, FramePrepStage::Advance, &mut sub);

    let timings_write_time = {
        let t = Instant::now();
        gpu_timings_write(&mut ctx.world);
        t.elapsed().as_secs_f32() * 1000.0
    };
    sub.insert("timings_write".to_string(), timings_write_time);

    run_frame_prep_hooks(ctx, FramePrepStage::Accumulate, &mut sub);

    let t = Instant::now();
    let render_data_vec = collect_gizmo_render_data(ctx, camera_position);
    sub.insert(
        "collect_gizmo".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );
    let render_data_refs: Vec<_> = render_data_vec.iter().collect();

    let t = Instant::now();
    if let Err(e) = update_object_ubo(
        &render_data_refs,
        ctx.image_index,
        &ctx.graphics.objects,
        ctx.device,
    ) {
        eprintln!("Failed to update object UBOs: {}", e);
    }
    sub.insert("object_ubo".to_string(), t.elapsed().as_secs_f32() * 1000.0);

    let t = Instant::now();
    update_billboard_ubo(ctx, view, proj)?;
    sub.insert(
        "billboard_ubo".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    update_transform_gizmo_mesh(ctx)?;
    sub.insert(
        "transform_gizmo".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    update_bone_gizmo_mesh(ctx)?;
    sub.insert("bone_gizmo".to_string(), t.elapsed().as_secs_f32() * 1000.0);

    let t = Instant::now();
    update_constraint_gizmo_mesh(ctx)?;
    sub.insert(
        "constraint_gizmo".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    update_spring_bone_gizmo_mesh(ctx)?;
    sub.insert(
        "spring_gizmo".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    upload_light_gizmo_if_changed(ctx)?;
    sub.insert(
        "light_gizmo".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let t = Instant::now();
    crate::ecs::systems::gizmo_systems::run_vertical_lines_update(ctx)?;
    sub.insert(
        "vertical_lines".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    ctx.world
        .resource_mut::<crate::ecs::resource::PhaseSubTimings>()
        .phases
        .insert("render_prep", sub);

    crate::ecs::systems::raytracing_systems::refresh_tlas_mesh_transforms(ctx)?;

    Ok(())
}

unsafe fn upload_light_gizmo_if_changed(ctx: &mut FrameContext) -> Result<()> {
    if ctx.light_gizmo().pending_uploads == 0 {
        return Ok(());
    }

    let mut mesh = ctx.light_gizmo().mesh.clone();
    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut mesh, frame_slot)?;
    }

    let mut light_gizmo = ctx.light_gizmo_mut();
    light_gizmo.mesh = mesh;
    light_gizmo.pending_uploads -= 1;

    Ok(())
}

fn run_frame_prep_hooks(
    ctx: &mut FrameContext,
    stage: FramePrepStage,
    sub: &mut HashMap<String, f32>,
) {
    let hooks = ctx
        .world
        .get_resource::<FramePrepHooks>()
        .map(|hooks| hooks.at_stage(stage))
        .unwrap_or_default();
    for hook in hooks {
        let t = Instant::now();
        (hook.run)(ctx);
        sub.insert(hook.timing_key(), t.elapsed().as_secs_f32() * 1000.0);
    }
}

unsafe fn update_mesh_entity_transforms(ctx: &mut FrameContext) -> Result<()> {
    use crate::ecs::world::{GlobalTransform, MeshRef};
    use crate::render::ObjectUBO;

    let transforms: Vec<(usize, Matrix4<f32>)> = ctx
        .world
        .iter_components::<MeshRef>()
        .map(|(entity, mesh_ref)| {
            let model_matrix = ctx
                .world
                .get_component::<GlobalTransform>(entity)
                .map(|gt| gt.0)
                .unwrap_or_else(Matrix4::identity);
            (mesh_ref.object_index, model_matrix)
        })
        .collect();

    for (object_index, model_matrix) in transforms {
        let ubo = ObjectUBO {
            model: model_matrix,
        };
        ctx.graphics
            .objects
            .update(ctx.device, ctx.image_index, object_index, &ubo)?;
    }

    Ok(())
}

unsafe fn update_frame_and_scene_uniforms(
    ctx: &mut FrameContext,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
    screen_size: cgmath::Vector2<f32>,
    aspect: f32,
    camera_position: Vector3<f32>,
) -> Result<()> {
    let light_position = ctx.light_state().light_position;

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

    update_mesh_entity_transforms(ctx)?;

    let light = ctx.light_state();
    let light_pos = light.light_position;
    let shadow_strength = light.shadow_strength;
    let distance_attenuation = light.distance_attenuation;
    drop(light);

    let debug_mode = ctx.debug_view_state().debug_view_mode.as_int();

    let brightness = ctx
        .world
        .get_resource::<SceneBrightness>()
        .map(|b| *b)
        .unwrap_or_default();
    let exposure_value = ctx
        .world
        .get_resource::<Exposure>()
        .map(|e| e.display_multiplier(brightness))
        .unwrap_or(1.0);

    let mut backend = ctx.create_backend();
    backend.update_scene_uniform(
        view,
        proj,
        light_pos,
        thyllore_render_core::DIRECT_LIGHT_COLOR,
        debug_mode,
        shadow_strength,
        distance_attenuation,
        exposure_value,
    )?;

    Ok(())
}

fn collect_gizmo_render_data(
    ctx: &FrameContext,
    camera_position: Vector3<f32>,
) -> Vec<crate::ecs::component::RenderData> {
    let frame_slot = ctx.frame_slot;
    let mut render_data_vec = vec![
        grid_mesh_render_data(&ctx.grid_mesh()),
        gizmo_mesh_render_data(&ctx.gizmo()),
        gizmo_selectable_render_data(&ctx.light_gizmo(), camera_position, frame_slot),
    ];

    if ctx.world.contains_resource::<BoneGizmoData>() {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        if bone_gizmo.visible {
            render_data_vec.extend(bone_gizmo_render_data(&bone_gizmo, frame_slot));
        }
    }

    if ctx.world.contains_resource::<ConstraintGizmoData>() {
        let cg = ctx.world.resource::<ConstraintGizmoData>();
        if cg.visible {
            render_data_vec.extend(constraint_gizmo_render_data(&cg, frame_slot));
        }
    }

    if ctx.world.contains_resource::<SpringBoneGizmoData>() {
        let sg = ctx.world.resource::<SpringBoneGizmoData>();
        if sg.draws_anything() {
            render_data_vec.extend(spring_bone_gizmo_render_data(&sg, frame_slot));
        }
    }

    if ctx.world.contains_resource::<TransformGizmoData>() {
        let tg = ctx.world.resource::<TransformGizmoData>();
        let gizmo_scale = ctx
            .world
            .get_resource::<TransformGizmoState>()
            .map(|s| s.gizmo_scale)
            .unwrap_or(0.08);
        render_data_vec.extend(transform_gizmo_render_data(
            &tg,
            camera_position,
            gizmo_scale,
            frame_slot,
        ));
    }

    render_data_vec
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
