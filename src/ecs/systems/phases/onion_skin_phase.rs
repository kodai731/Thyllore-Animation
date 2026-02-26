use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{ClipLibrary, OnionSkinningConfig, TimelineState};
use crate::ecs::systems::onion_skinning_systems::{
    compute_onion_skin_ghosts, OnionSkinMeshContext,
};

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

    let mesh_ctx = OnionSkinMeshContext {
        base_vertices: &base_vertices,
        mesh_index,
        skin_data: &skin_data,
    };

    let result = {
        let clip_library = ctx.world.resource::<ClipLibrary>();
        compute_onion_skin_ghosts(
            &config,
            current_time,
            ctx.world,
            ctx.assets,
            &clip_library,
            &mesh_ctx,
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
