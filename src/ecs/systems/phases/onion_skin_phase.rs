use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{ClipLibrary, HierarchyState, OnionSkinningConfig, TimelineState};
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
            log!("[onion_phase] SKIP: config not enabled");
            clear_ghost_buffers(ctx);
            return Ok(());
        }
    };

    let selected = ctx
        .world
        .get_resource::<HierarchyState>()
        .and_then(|h| h.selected_entity);
    let found_mesh_index = find_selected_mesh_index(ctx);
    log!(
        "[onion_phase] selected_entity={:?}, found_mesh_index={:?}",
        selected,
        found_mesh_index
    );
    let mesh_index = match found_mesh_index {
        Some(idx) if updated_meshes.contains(&idx) => idx,
        _ => {
            log!(
                "[onion_phase] SKIP: mesh_index={:?}, updated_meshes={:?}",
                found_mesh_index,
                updated_meshes
            );
            clear_ghost_buffers(ctx);
            return Ok(());
        }
    };

    let (skin_data, base_vertices) = {
        let mesh = &ctx.graphics.meshes[mesh_index];
        match &mesh.skin_data {
            Some(sd) => (sd.clone(), mesh.base_vertices.clone()),
            None => {
                log!(
                    "[onion_phase] SKIP: no skin_data for mesh_index={}",
                    mesh_index
                );
                clear_ghost_buffers(ctx);
                return Ok(());
            }
        }
    };

    if base_vertices.len() != skin_data.base_positions.len() {
        log!(
            "[onion_phase] MISMATCH: base_vertices.len()={} != skin_data.base_positions.len()={}",
            base_vertices.len(),
            skin_data.base_positions.len(),
        );
        clear_ghost_buffers(ctx);
        return Ok(());
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
    log!(
        "[onion_phase] ghost_count={}, mesh_index={}, time={:.4}",
        ghost_count,
        mesh_index,
        current_time
    );
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
    }

    log!(
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

    if let Some(mesh_ref) = ctx.world.get_component::<MeshRef>(entity) {
        let mesh_asset = ctx.assets.get_mesh(mesh_ref.mesh_asset_id)?;
        return Some(mesh_asset.graphics_mesh_index);
    }

    let children = ctx.world.find_child_mesh_entities(entity);
    log!(
        "[onion_phase] find_selected_mesh_index: entity={}, has_MeshRef=false, child_meshes={}",
        entity,
        children.len()
    );

    let first_child = children.into_iter().next()?;
    let mesh_ref = ctx.world.get_component::<MeshRef>(first_child)?;
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
