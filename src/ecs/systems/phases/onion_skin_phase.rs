use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{
    ClipLibrary, HierarchyState, OnionSkinningConfig, TimelineState,
};
use crate::ecs::systems::onion_skinning_systems::{
    compute_onion_skin_ghosts, OnionSkinMeshContext,
};
use crate::ecs::world::MeshRef;

pub unsafe fn run_onion_skin_phase(
    ctx: &mut FrameContext,
    updated_meshes: &[usize],
) -> Result<()> {
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

    let (skin_data, base_vertices) = {
        let mesh = &ctx.graphics.meshes[mesh_index];
        match &mesh.skin_data {
            Some(sd) => (sd.clone(), mesh.base_vertices.clone()),
            None => {
                clear_ghost_buffers(ctx);
                return Ok(());
            }
        }
    };

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
