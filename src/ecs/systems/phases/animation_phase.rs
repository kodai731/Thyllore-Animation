use anyhow::Result;

use crate::app::FrameContext;
use crate::debugview::gizmo::BoneGizmoData;
use crate::ecs::resource::{ClipLibrary, NodeAssets, OnionSkinningConfig, TimelineState};
use crate::ecs::systems::onion_skinning_systems::compute_onion_skin_ghosts;
use crate::ecs::{
    evaluate_all_animators, playback_upload_animations, transform_propagation_system,
};

pub struct AnimationUpdates {
    pub updated_meshes: Vec<usize>,
}

pub fn run_animation_phase_ecs(ctx: &mut FrameContext) -> AnimationUpdates {
    let eval_result = {
        let clip_library = ctx.world.resource::<ClipLibrary>();
        let mut node_assets = ctx.world.resource_mut::<NodeAssets>();

        evaluate_all_animators(
            ctx.world,
            ctx.graphics,
            &mut node_assets.nodes,
            &*clip_library,
            ctx.assets,
            ctx.delta_time,
        )
    };

    if let Some((skel_id, transforms)) = &eval_result.bone_transforms {
        if ctx.world.contains_resource::<BoneGizmoData>() {
            let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
            bone_gizmo.cached_skeleton_id = Some(*skel_id);
            bone_gizmo.cached_global_transforms = transforms.clone();
        }
    }

    transform_propagation_system(ctx.world);

    AnimationUpdates {
        updated_meshes: eval_result.updated_meshes,
    }
}

pub unsafe fn run_animation_phase_gpu(
    ctx: &mut FrameContext,
    updates: &AnimationUpdates,
) -> Result<()> {
    if !updates.updated_meshes.is_empty() {
        let mut backend = ctx.create_backend();
        playback_upload_animations(&mut backend, &updates.updated_meshes)?;
    }

    Ok(())
}

pub unsafe fn run_onion_skin_phase(
    ctx: &mut FrameContext,
    updated_meshes: &[usize],
) -> Result<()> {
    let config = match ctx.world.get_resource::<OnionSkinningConfig>() {
        Some(c) if c.enabled => (*c).clone(),
        _ => {
            if let Some(ref mut gpu) = *ctx.onion_skin_gpu {
                for buffer in &mut gpu.ghost_buffers {
                    buffer.vertex_count = 0;
                }
            }
            return Ok(());
        }
    };

    let mesh_data = {
        let mut found = None;
        for &idx in updated_meshes {
            if idx < ctx.graphics.meshes.len() {
                let mesh = &ctx.graphics.meshes[idx];
                if let Some(ref sd) = mesh.skin_data {
                    found = Some((idx, sd.clone(), mesh.base_vertices.clone()));
                    break;
                }
            }
        }
        match found {
            Some(d) => d,
            None => return Ok(()),
        }
    };

    let (mesh_index, skin_data, base_vertices) = mesh_data;

    let current_time = ctx
        .world
        .get_resource::<TimelineState>()
        .map(|ts| ts.current_time)
        .unwrap_or(0.0);

    let result = {
        let clip_library = ctx.world.resource::<ClipLibrary>();
        compute_onion_skin_ghosts(
            &config,
            current_time,
            ctx.world,
            ctx.assets,
            &clip_library,
            &base_vertices,
            mesh_index,
            &skin_data,
        )
    };

    let ghost_count = result.ghost_meshes.len();
    if ghost_count == 0 {
        return Ok(());
    }

    let vertex_capacity = base_vertices.len();

    let gpu = ctx
        .onion_skin_gpu
        .get_or_insert_with(Default::default);

    gpu.ensure_capacity(ctx.instance, ctx.device, ghost_count, vertex_capacity)?;

    let mesh = &ctx.graphics.meshes[mesh_index];
    gpu.source_index_buffer = mesh.index_buffer.buffer;
    gpu.source_index_count = mesh.index_buffer.indices;
    gpu.source_mesh_index = Some(mesh_index);

    for (i, ghost) in result.ghost_meshes.iter().enumerate() {
        if i < gpu.ghost_buffers.len() {
            gpu.ghost_buffers[i].update_vertices(
                ctx.device,
                &ghost.vertices,
                ghost.tint_color,
                ghost.opacity,
            )?;
        }
    }

    log::debug!(
        "Onion skin: {} ghosts computed for mesh {}",
        ghost_count,
        mesh_index
    );

    Ok(())
}
