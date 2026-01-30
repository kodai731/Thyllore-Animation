use anyhow::Result;
use cgmath::Vector3;

use crate::app::FrameContext;
use crate::ecs::context::EcsContext;
use crate::ecs::resource::{AnimationPlayback, ClipLibrary, HierarchyState, TimelineState};
use crate::ecs::world::Animator;
use crate::app::graphics_resource::GraphicsResources;
use super::phases::{
    run_animation_phase_ecs, run_animation_phase_gpu, run_input_phase, run_render_prep_phase,
    run_transform_phase_ecs, run_transform_phase_gpu,
};
use super::timeline_systems::timeline_update;

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

    run_timeline_phase(ctx);

    let animation_updates = run_animation_phase_ecs(ctx);
    run_animation_phase_gpu(ctx, &animation_updates)?;

    run_transform_phase_gpu(ctx)?;
    run_render_prep_phase(ctx)?;
    Ok(())
}

fn run_timeline_phase(ctx: &mut FrameContext) {
    if !ctx.world.contains_resource::<TimelineState>() {
        return;
    }
    if !ctx.world.contains_resource::<ClipLibrary>() {
        return;
    }

    let selected_entity = {
        let hierarchy_state = ctx.world.resource::<HierarchyState>();
        hierarchy_state.selected_entity
    };

    {
        let mut timeline_state = ctx.world.resource_mut::<TimelineState>();
        timeline_state.target_entity = selected_entity;
    }

    let mut timeline_state = ctx.world.resource_mut::<TimelineState>();
    let mut playback = ctx.world.resource_mut::<AnimationPlayback>();
    let clip_library = ctx.world.resource::<ClipLibrary>();
    timeline_update(&mut timeline_state, &mut playback, &*clip_library, ctx.delta_time);
    drop(clip_library);
    drop(playback);
    drop(timeline_state);

    sync_playback_to_animator(ctx, selected_entity);

    sync_editable_clips_to_registry(ctx);
}

fn sync_playback_to_animator(ctx: &mut FrameContext, selected_entity: Option<u64>) {
    let entity = selected_entity.or_else(|| {
        ctx.world
            .iter_animated_entities()
            .next()
            .map(|(e, _)| e)
    });

    let Some(entity) = entity else {
        return;
    };

    let playback_snapshot = {
        let playback = ctx.world.resource::<AnimationPlayback>();
        (
            playback.time,
            playback.playing,
            playback.speed,
            playback.looping,
            playback.current_clip_id,
        )
    };

    if let Some(animator) = ctx.world.get_component_mut::<Animator>(entity) {
        animator.time = playback_snapshot.0;
        animator.playing = playback_snapshot.1;
        animator.speed = playback_snapshot.2;
        animator.looping = playback_snapshot.3;
        animator.current_clip_id = playback_snapshot.4;
    }
}

fn sync_editable_clips_to_registry(ctx: &mut FrameContext) {
    let mut clip_library = ctx.world.resource_mut::<ClipLibrary>();
    clip_library.sync_dirty_clips();
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
