use anyhow::Result;

use crate::app::FrameContext;
use crate::ecs::resource::{ClipLibrary, HierarchyState, OnionSkinningConfig, TimelineState};
use crate::ecs::systems::onion_skinning_systems::{
    compute_onion_skin_ghosts, OnionSkinMeshContext,
};
use crate::ecs::world::MeshRef;

pub unsafe fn run_onion_skin_phase(ctx: &mut FrameContext, updated_meshes: &[usize]) -> Result<()> {
    let Some(config) = resolve_onion_config(ctx) else {
        clear_ghost_buffers(ctx);
        return Ok(());
    };

    let Some(mesh_index) = resolve_target_mesh(ctx, updated_meshes) else {
        clear_ghost_buffers(ctx);
        return Ok(());
    };

    let Some((skin_data, base_vertices)) = extract_skin_data(ctx, mesh_index) else {
        clear_ghost_buffers(ctx);
        return Ok(());
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

    if result.ghost_meshes.is_empty() {
        return Ok(());
    }

    upload_ghost_meshes(ctx, &result.ghost_meshes, mesh_index, base_vertices.len())
}

fn resolve_onion_config(ctx: &FrameContext) -> Option<OnionSkinningConfig> {
    ctx.world
        .get_resource::<OnionSkinningConfig>()
        .filter(|c| c.enabled)
        .map(|c| (*c).clone())
}

fn resolve_target_mesh(ctx: &FrameContext, updated_meshes: &[usize]) -> Option<usize> {
    let mesh_index = find_selected_mesh_index(ctx)?;
    updated_meshes.contains(&mesh_index).then_some(mesh_index)
}

fn extract_skin_data(
    ctx: &FrameContext,
    mesh_index: usize,
) -> Option<(
    crate::animation::SkinData,
    Vec<crate::vulkanr::data::Vertex>,
)> {
    let mesh = &ctx.graphics.meshes[mesh_index];
    let skin_data = mesh.skin_data.as_ref()?;
    let base_vertices = &mesh.base_vertices;

    if base_vertices.len() != skin_data.base_positions.len() {
        log_warn!(
            "onion skin: base_vertices({}) != base_positions({})",
            base_vertices.len(),
            skin_data.base_positions.len(),
        );
        return None;
    }

    Some((skin_data.clone(), base_vertices.clone()))
}

fn find_selected_mesh_index(ctx: &FrameContext) -> Option<usize> {
    let hierarchy = ctx.world.get_resource::<HierarchyState>()?;
    let entity = hierarchy.selected_entity?;

    if let Some(mesh_ref) = ctx.world.get_component::<MeshRef>(entity) {
        let mesh_asset = ctx.assets.get_mesh(mesh_ref.mesh_asset_id)?;
        return Some(mesh_asset.graphics_mesh_index);
    }

    let first_child = ctx
        .world
        .find_child_mesh_entities(entity)
        .into_iter()
        .next()?;
    let mesh_ref = ctx.world.get_component::<MeshRef>(first_child)?;
    let mesh_asset = ctx.assets.get_mesh(mesh_ref.mesh_asset_id)?;
    Some(mesh_asset.graphics_mesh_index)
}

unsafe fn upload_ghost_meshes(
    ctx: &mut FrameContext,
    ghost_meshes: &[crate::ecs::resource::GhostMeshData],
    mesh_index: usize,
    vertex_capacity: usize,
) -> Result<()> {
    let gpu = ctx.onion_skin_gpu.get_or_insert_with(Default::default);
    gpu.ensure_capacity(
        ctx.instance,
        ctx.device,
        ghost_meshes.len(),
        vertex_capacity,
    )?;

    let mesh = &ctx.graphics.meshes[mesh_index];
    gpu.source_index_buffer = mesh.index_buffer.buffer;
    gpu.source_index_count = mesh.index_buffer.indices;
    gpu.source_mesh_index = Some(mesh_index);

    for (i, ghost) in ghost_meshes.iter().enumerate() {
        if i < gpu.ghost_buffers.len() {
            gpu.ghost_buffers[i].update_vertices(
                ctx.device,
                &ghost.vertices,
                ghost.tint_color,
                ghost.opacity,
            )?;
        }
    }

    Ok(())
}

fn clear_ghost_buffers(ctx: &mut FrameContext) {
    if let Some(ref mut gpu) = *ctx.onion_skin_gpu {
        for buffer in &mut gpu.ghost_buffers {
            buffer.vertex_count = 0;
        }
    }
}
