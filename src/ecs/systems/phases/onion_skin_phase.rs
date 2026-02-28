use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{
    ClipLibrary, GhostMeshData, HierarchyState, OnionSkinningConfig, TimelineState,
};
use crate::ecs::systems::onion_skinning_systems::{
    compute_onion_skin_ghosts, OnionSkinMeshContext,
};
use crate::ecs::world::MeshRef;

pub unsafe fn run_onion_skin_phase(ctx: &mut FrameContext, updated_meshes: &[usize]) -> Result<()> {
    let config = ctx
        .world
        .get_resource::<OnionSkinningConfig>()
        .filter(|c| c.enabled)
        .map(|c| (*c).clone());

    let config = match config {
        Some(c) => c,
        None => {
            clear_ghost_buffers(ctx);
            return Ok(());
        }
    };

    let mesh_index = match find_selected_mesh_index(ctx) {
        Some(idx) if updated_meshes.contains(&idx) => idx,
        _ => {
            clear_ghost_buffers(ctx);
            return Ok(());
        }
    };

    let (skin_data, base_vertices, vtx_buf_len, index_count, max_index) = {
        let mesh = &ctx.graphics.meshes[mesh_index];
        match &mesh.skin_data {
            Some(sd) => {
                let max_idx = mesh.vertex_data.indices.iter().copied().max().unwrap_or(0);
                (
                    sd.clone(),
                    mesh.base_vertices.clone(),
                    mesh.vertex_data.vertices.len(),
                    mesh.vertex_data.indices.len(),
                    max_idx,
                )
            }
            None => {
                clear_ghost_buffers(ctx);
                return Ok(());
            }
        }
    };

    crate::log!(
        "[onion_phase] mesh_index={}, base_vertices={}, vtx_buf={}, indices={}, max_index={}, skin_positions={}",
        mesh_index,
        base_vertices.len(),
        vtx_buf_len,
        index_count,
        max_index,
        skin_data.base_positions.len(),
    );

    if base_vertices.len() != skin_data.base_positions.len() {
        crate::log!(
            "[onion_phase] MISMATCH: base_vertices.len()={} != skin_data.base_positions.len()={}",
            base_vertices.len(),
            skin_data.base_positions.len(),
        );
    }

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

    let gpu = ctx.onion_skin_gpu.get_or_insert_with(Default::default);

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

        crate::log!(
            "[onion_gpu] ghost[{}] verts={}, bounds=({:.3},{:.3},{:.3})..({:.3},{:.3},{:.3}), zero_weight={}, near_origin={}",
            i,
            ghost.vertices.len(),
            ghost.diag_bounds_min[0], ghost.diag_bounds_min[1], ghost.diag_bounds_min[2],
            ghost.diag_bounds_max[0], ghost.diag_bounds_max[1], ghost.diag_bounds_max[2],
            ghost.diag_zero_weight_count,
            ghost.diag_near_origin_count,
        );

        log_sample_vertices(ghost);
    }

    crate::log!(
        "[onion_phase] {} ghosts uploaded, index_count={}, time={:.4}",
        ghost_count,
        gpu.source_index_count,
        current_time,
    );

    Ok(())
}

fn find_selected_mesh_index(ctx: &FrameContext) -> Option<usize> {
    let hierarchy = ctx.world.get_resource::<HierarchyState>()?;
    let entity = hierarchy.selected_entity?;
    let mesh_ref = ctx.world.get_component::<MeshRef>(entity)?;
    let mesh_asset = ctx.assets.get_mesh(mesh_ref.mesh_asset_id)?;
    Some(mesh_asset.graphics_mesh_index)
}

fn clear_ghost_buffers(ctx: &mut FrameContext) {
    if let Some(ref mut gpu) = *ctx.onion_skin_gpu {
        for buffer in &mut gpu.ghost_buffers {
            buffer.vertex_count = 0;
        }
    }
}

fn log_sample_vertices(ghost: &GhostMeshData) {
    let verts = &ghost.vertices;
    if verts.is_empty() {
        return;
    }

    let near_origin_threshold = 0.01;
    let mut near_origin_samples: Vec<(usize, [f32; 3])> = Vec::new();
    let mut spread_samples: Vec<(usize, [f32; 3])> = Vec::new();

    let step = (verts.len() / 8).max(1);

    for (i, v) in verts.iter().enumerate() {
        let pos = [v.pos.x, v.pos.y, v.pos.z];
        let dist = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();

        if dist < near_origin_threshold && near_origin_samples.len() < 5 {
            near_origin_samples.push((i, pos));
        }

        if i % step == 0 && spread_samples.len() < 8 {
            spread_samples.push((i, pos));
        }
    }

    if !near_origin_samples.is_empty() {
        for (idx, pos) in &near_origin_samples {
            crate::log!(
                "[onion_gpu]   near_origin v[{}] pos=({:.6},{:.6},{:.6})",
                idx,
                pos[0],
                pos[1],
                pos[2],
            );
        }
    }

    for (idx, pos) in &spread_samples {
        crate::log!(
            "[onion_gpu]   sample v[{}] pos=({:.4},{:.4},{:.4})",
            idx,
            pos[0],
            pos[1],
            pos[2],
        );
    }
}
