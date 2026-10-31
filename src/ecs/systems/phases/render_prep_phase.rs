use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::gizmo::BoneSelectionState;
use crate::ecs::resource::gizmo::TransformGizmoData;
use crate::ecs::resource::gizmo::{
    BoneDisplayStyle, BoneGizmoData, ConstraintGizmoData, SpringBoneGizmoData, SpringChainGizmoMode,
};
use crate::ecs::resource::HierarchyState;
use crate::ecs::resource::ProjectionData;
use crate::ecs::resource::{
    Camera, Exposure, GpuPassTimings, GpuTimingsSink, SceneBrightness, TransformGizmoState,
};
use crate::ecs::systems::render_data_systems::{
    bone_gizmo_render_data, constraint_gizmo_render_data, gizmo_mesh_render_data,
    gizmo_selectable_render_data, grid_mesh_render_data, spring_bone_gizmo_render_data,
    transform_gizmo_render_data,
};
use crate::ecs::systems::{
    chains_skinned_to_entity, compute_bone_draw_set, spring_chain_bone_indices, BoneDrawSet,
    ColliderGizmo, SpringGizmoSelection,
};
use crate::ecs::world::World;
use crate::ecs::FrameContext;
use crate::ecs::{
    build_bone_line_mesh, build_box_bone_meshes_with_selection, build_constraint_gizmo_mesh,
    build_octahedral_bone_meshes_with_selection, build_sphere_bone_meshes_with_selection,
    build_spring_bone_gizmo_mesh,
};
use crate::hooks::frame_prep::{FramePrepHooks, FramePrepStage};
use crate::render::RenderBackend;
use crate::vulkanr::renderer::scene_renderer::update_object_ubo;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_anim_core::spring_bone::{chains_to_draw, chains_with_radius, SpringChainId};
use thyllore_math_core::ScreenSpaceScale;

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

fn gpu_timings_write(world: &mut crate::ecs::World) {
    let timings = match world.get_resource::<GpuPassTimings>() {
        Some(t) => t,
        None => return,
    };
    let frame = timings.frame;
    let frame_total_ms = timings.frame_total_ms;
    let passes: Vec<(String, f32)> = timings.passes.clone();
    if passes.is_empty() {
        return;
    }
    let mut sink = match world.get_resource_mut::<GpuTimingsSink>() {
        Some(s) => s,
        None => return,
    };
    if frame == sink.last_frame {
        return;
    }
    let passes_map: serde_json::Map<String, serde_json::Value> = passes
        .into_iter()
        .map(|(label, ms)| (label, serde_json::json!(ms)))
        .collect();

    let mut obj: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    obj.insert("frame".to_string(), serde_json::json!(frame));
    obj.insert("passes".to_string(), serde_json::Value::Object(passes_map));
    if let Some(ms) = frame_total_ms {
        obj.insert("frame_total_ms".to_string(), serde_json::json!(ms));
    }

    if let Some(cpu) = world.get_resource::<crate::ecs::resource::CpuFrameTimings>() {
        obj.insert("cpu_dt_ms".to_string(), serde_json::json!(cpu.dt_ms));
        obj.insert("imgui_vtx".to_string(), serde_json::json!(cpu.imgui_vtx));
        obj.insert("imgui_idx".to_string(), serde_json::json!(cpu.imgui_idx));
        let cpu_map: serde_json::Map<String, serde_json::Value> = cpu
            .stages
            .iter()
            .map(|(label, ms)| (label.clone(), serde_json::json!(ms)))
            .collect();
        obj.insert("cpu".to_string(), serde_json::Value::Object(cpu_map));
    }
    if let Some(up) = world.get_resource::<crate::ecs::resource::UpdatePhaseTimings>() {
        let up_map: serde_json::Map<String, serde_json::Value> = up
            .stages
            .iter()
            .map(|(label, ms)| (label.clone(), serde_json::json!(ms)))
            .collect();
        obj.insert(
            "update_phases".to_string(),
            serde_json::Value::Object(up_map),
        );
    }
    if let Some(phase_timings) = world.get_resource::<crate::ecs::resource::PhaseSubTimings>() {
        for (phase_name, timings) in &phase_timings.phases {
            let map: serde_json::Map<String, serde_json::Value> = timings
                .iter()
                .map(|(label, ms)| (label.clone(), serde_json::json!(ms)))
                .collect();
            obj.insert(
                format!("{}_sub", phase_name),
                serde_json::Value::Object(map),
            );
        }
    }

    let line = serde_json::Value::Object(obj);
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&sink.path)
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{}", line)
        })
    {
        eprintln!("gpu timings write failed: {}", e);
    }
    sink.last_frame = frame;
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

unsafe fn update_transform_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<TransformGizmoData>() {
        return Ok(());
    }

    let visible = {
        let tg = ctx.world.resource::<TransformGizmoData>();
        tg.visible
    };

    if !visible {
        return Ok(());
    }

    let (mode, active_handle) = {
        let tg = ctx.world.resource::<TransformGizmoData>();
        let state = ctx
            .world
            .resource::<crate::ecs::resource::TransformGizmoState>();
        (state.mode, tg.active_handle)
    };

    let camera_dir = ctx.camera_direction();

    let (mut line_mesh_clone, mut solid_mesh_clone) = {
        let tg = ctx.world.resource::<TransformGizmoData>();
        let line_mesh_clone = tg.line_mesh.clone();
        let solid_mesh_clone = tg.solid_mesh.clone();
        drop(tg);
        (line_mesh_clone, solid_mesh_clone)
    };

    match mode {
        crate::ecs::resource::TransformGizmoMode::Translate => {
            crate::ecs::systems::transform_gizmo_systems::build_translate_gizmo_meshes(
                active_handle,
                &mut line_mesh_clone,
                &mut solid_mesh_clone,
            );
        }
        crate::ecs::resource::TransformGizmoMode::Rotate => {
            crate::ecs::systems::transform_gizmo_systems::build_rotate_gizmo_meshes(
                active_handle,
                camera_dir,
                &mut line_mesh_clone,
                &mut solid_mesh_clone,
            );
        }
        crate::ecs::resource::TransformGizmoMode::Scale => {
            crate::ecs::systems::transform_gizmo_systems::build_scale_gizmo_meshes(
                active_handle,
                &mut line_mesh_clone,
                &mut solid_mesh_clone,
            );
        }
    }

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut line_mesh_clone, frame_slot)?;
        backend.update_or_create_line_buffers(&mut solid_mesh_clone, frame_slot)?;
    }

    {
        let mut tg = ctx.world.resource_mut::<TransformGizmoData>();
        tg.line_mesh = line_mesh_clone;
        tg.solid_mesh = solid_mesh_clone;
    }

    Ok(())
}

unsafe fn update_bone_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let (
        visible,
        display_style,
        draw_mode,
        skeleton_id,
        transforms,
        offsets,
        distance_scaling_enabled,
        distance_scaling_factor,
        mesh_scale,
    ) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.visible,
            bone_gizmo.display_style,
            bone_gizmo.draw_mode,
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
            bone_gizmo.distance_scaling_enabled,
            bone_gizmo.distance_scaling_factor,
            bone_gizmo.mesh_scale,
        )
    };

    if !visible {
        ctx.world
            .resource_mut::<BoneGizmoData>()
            .drawn_bones
            .clear();
        return Ok(());
    }

    let Some(skel_id) = skeleton_id else {
        return Ok(());
    };

    let Some(skeleton) = ctx.assets.get_skeleton_by_skeleton_id(skel_id) else {
        return Ok(());
    };
    let skeleton = skeleton.clone();

    let visual_scale = compute_visual_scale(
        ctx,
        &transforms,
        distance_scaling_enabled,
        distance_scaling_factor,
    );

    let selection = bone_selection_snapshot(ctx.world);
    let chain_bones = find_spring_bone_setup(ctx.world)
        .map(|setup| spring_chain_bone_indices(&setup))
        .unwrap_or_default();
    let draw_set = compute_bone_draw_set(&skeleton, draw_mode, &selection, &chain_bones);
    ctx.world.resource_mut::<BoneGizmoData>().drawn_bones = draw_set.flags().to_vec();

    match display_style {
        BoneDisplayStyle::Stick => {
            let marker_scale = joint_marker_scale(ctx);
            update_stick_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                mesh_scale,
                &draw_set,
                &marker_scale,
            )?;
        }
        BoneDisplayStyle::Octahedral => {
            update_octahedral_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                visual_scale,
                mesh_scale,
                &draw_set,
            )?;
        }
        BoneDisplayStyle::Box => {
            update_box_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                visual_scale,
                mesh_scale,
                &draw_set,
            )?;
        }
        BoneDisplayStyle::Sphere => {
            update_sphere_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                visual_scale,
                mesh_scale,
                &draw_set,
            )?;
        }
    }

    Ok(())
}

fn bone_selection_snapshot(world: &World) -> BoneSelectionState {
    world
        .get_resource::<BoneSelectionState>()
        .map(|selection| (*selection).clone())
        .unwrap_or_default()
}

/// A selected skinned mesh (hair, skirt) shows the chains that move it, so the chains are found
/// without selecting their root bone, and a viewport click that clears the bone selection keeps them.
fn chains_of_selected_mesh(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    setup: &SpringBoneSetup,
) -> Vec<SpringChainId> {
    let Some(entity) = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity)
    else {
        return Vec::new();
    };
    chains_skinned_to_entity(world, assets, graphics, setup, entity)
}

/// The spring gizmo accompanies the bone gizmo: chains and colliders show only while a bone is drawn
/// or a skinned mesh is selected.
fn any_bone_drawn(world: &World) -> bool {
    world
        .resource::<BoneGizmoData>()
        .drawn_bones
        .iter()
        .any(|&drawn| drawn)
}

fn find_spring_bone_setup(world: &World) -> Option<SpringBoneSetup> {
    let (entity, _) = world.iter_components::<WithSpringBone>().next()?;
    world.get_component::<SpringBoneSetup>(entity).cloned()
}

fn joint_marker_scale(ctx: &FrameContext) -> ScreenSpaceScale {
    use crate::ecs::systems::camera_systems::compute_camera_position;
    let camera = ctx.world.resource::<Camera>();
    let viewport_height = ctx.world.resource::<ProjectionData>().screen_size.y;
    ScreenSpaceScale::new(
        compute_camera_position(&camera),
        cgmath::Rad::from(camera.fov_y).0,
        viewport_height,
    )
}

fn compute_visual_scale(
    ctx: &FrameContext,
    transforms: &[Matrix4<f32>],
    distance_scaling_enabled: bool,
    distance_scaling_factor: f32,
) -> f32 {
    if !distance_scaling_enabled || transforms.is_empty() {
        return 1.0;
    }

    let camera_pos = {
        use crate::ecs::systems::camera_systems::compute_camera_position;
        compute_camera_position(&ctx.world.resource::<Camera>())
    };

    let mut center = Vector3::new(0.0f32, 0.0, 0.0);
    for t in transforms.iter() {
        center.x += t[3][0];
        center.y += t[3][1];
        center.z += t[3][2];
    }
    let count = transforms.len() as f32;
    center /= count;

    let distance = (center - camera_pos).magnitude();
    (distance * distance_scaling_factor).max(0.1)
}

unsafe fn update_stick_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
    marker_scale: &ScreenSpaceScale,
) -> Result<()> {
    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        build_bone_line_mesh(
            skeleton,
            transforms,
            offsets,
            mesh_scale,
            None,
            draw_set,
            marker_scale,
            &mut bone_gizmo.stick_mesh,
        );
    }

    let mut mesh_clone = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        bone_gizmo.stick_mesh.clone()
    };

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut mesh_clone, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.stick_mesh.vertex_buffer_handles = mesh_clone.vertex_buffer_handles;
        bone_gizmo.stick_mesh.index_buffer_handles = mesh_clone.index_buffer_handles;
    }

    Ok(())
}

unsafe fn update_octahedral_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    visual_scale: f32,
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
) -> Result<()> {
    let selection = bone_selection_snapshot(ctx.world);

    let (mut solid_mesh, mut wire_mesh) = {
        let bg = ctx.world.resource::<BoneGizmoData>();
        let solid_mesh = bg.solid_mesh.clone();
        let wire_mesh = bg.wire_mesh.clone();
        drop(bg);
        (solid_mesh, wire_mesh)
    };
    build_octahedral_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        visual_scale,
        mesh_scale,
        None,
        draw_set,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh, frame_slot)?;
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}

unsafe fn update_box_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    visual_scale: f32,
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
) -> Result<()> {
    let selection = bone_selection_snapshot(ctx.world);

    let (mut solid_mesh, mut wire_mesh) = {
        let bg = ctx.world.resource::<BoneGizmoData>();
        let solid_mesh = bg.solid_mesh.clone();
        let wire_mesh = bg.wire_mesh.clone();
        drop(bg);
        (solid_mesh, wire_mesh)
    };
    build_box_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        visual_scale,
        mesh_scale,
        None,
        draw_set,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh, frame_slot)?;
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}

unsafe fn update_sphere_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    visual_scale: f32,
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
) -> Result<()> {
    let selection = bone_selection_snapshot(ctx.world);

    let (mut solid_mesh, mut wire_mesh) = {
        let bg = ctx.world.resource::<BoneGizmoData>();
        let solid_mesh = bg.solid_mesh.clone();
        let wire_mesh = bg.wire_mesh.clone();
        drop(bg);
        (solid_mesh, wire_mesh)
    };

    build_sphere_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        visual_scale,
        mesh_scale,
        None,
        draw_set,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh, frame_slot)?;
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}

unsafe fn update_constraint_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<ConstraintGizmoData>() {
        return Ok(());
    }
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let visible = {
        let cg = ctx.world.resource::<ConstraintGizmoData>();
        cg.visible
    };
    if !visible {
        return Ok(());
    }

    let (skeleton_id, transforms, offsets, constraint_mesh_scale) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
            bone_gizmo.mesh_scale,
        )
    };

    let Some(skel_id) = skeleton_id else {
        return Ok(());
    };
    let Some(skeleton) = ctx.assets.get_skeleton_by_skeleton_id(skel_id) else {
        return Ok(());
    };
    let skeleton = skeleton.clone();

    let constraint_set = ctx
        .world
        .iter_constrained_entities()
        .next()
        .map(|(_, cs)| cs.clone());

    let Some(constraint_set) = constraint_set else {
        return Ok(());
    };

    let mut wire_mesh = ctx
        .world
        .resource::<ConstraintGizmoData>()
        .wire_mesh
        .clone();

    build_constraint_gizmo_mesh(
        &constraint_set,
        &skeleton,
        &transforms,
        &offsets,
        constraint_mesh_scale,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut cg = ctx.world.resource_mut::<ConstraintGizmoData>();
        cg.wire_mesh = wire_mesh;
    }

    Ok(())
}

unsafe fn update_spring_bone_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<SpringBoneGizmoData>() {
        return Ok(());
    }
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let (colliders_visible, chain_mode) = {
        let sg = ctx.world.resource::<SpringBoneGizmoData>();
        (sg.colliders_visible, sg.chain_mode)
    };
    if !colliders_visible && chain_mode == SpringChainGizmoMode::None {
        return Ok(());
    }

    let (skeleton_id, transforms, offsets, mesh_scale) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
            bone_gizmo.mesh_scale,
        )
    };

    let Some(setup) = find_spring_bone_setup(ctx.world) else {
        return Ok(());
    };
    let skeleton = skeleton_id.and_then(|id| ctx.assets.get_skeleton_by_skeleton_id(id));
    let Some(skeleton) = skeleton else {
        return Ok(());
    };

    let selection = bone_selection_snapshot(ctx.world);
    let bones_drawn = any_bone_drawn(ctx.world);
    let mut chain_ids = if bones_drawn {
        chains_to_draw(
            &setup,
            skeleton,
            &selection.selected_bone_indices,
            chain_mode,
        )
    } else {
        Vec::new()
    };
    if chain_mode != SpringChainGizmoMode::None {
        chain_ids.extend(chains_of_selected_mesh(
            ctx.world,
            ctx.assets,
            ctx.graphics,
            &setup,
        ));
        chain_ids.sort_unstable();
        chain_ids.dedup();
    }
    let colliders = if colliders_visible && (bones_drawn || !chain_ids.is_empty()) {
        ColliderGizmo::Shown
    } else {
        ColliderGizmo::Hidden
    };
    let shown = SpringGizmoSelection {
        chain_ids,
        radius_chain_ids: chains_with_radius(&setup, selection.active_bone_index, chain_mode),
        colliders,
    };

    let mut wire_mesh = ctx
        .world
        .resource::<SpringBoneGizmoData>()
        .wire_mesh
        .clone();

    build_spring_bone_gizmo_mesh(
        &setup,
        &shown,
        &transforms,
        &offsets,
        mesh_scale,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut sg = ctx.world.resource_mut::<SpringBoneGizmoData>();
        sg.wire_mesh = wire_mesh;
    }

    Ok(())
}
