use anyhow::Result;
use glam::Mat4;
use thyllore_vulkan_core::core::RRDevice;
use thyllore_vulkan_core::descriptor::shader_bindings::effect_trace;
use thyllore_vulkan_core::descriptor::{push_constant_range, EFFECT_TRACE};
use thyllore_vulkan_core::pipeline::{max_ray_recursion_depth, RRRayTracingPipeline};
use thyllore_vulkan_core::raytracing::{RRAccelerationStructure, RRBLAS};

use crate::asset::AssetStorage;
use crate::ecs::resource::EffectTraceGpuState;
use crate::ecs::systems::RREffectTraceDescriptorSet;
use crate::ecs::world::{GlobalTransform, MeshRef, World};
use crate::ecs::{FrameContext, GpuSceneContext};
use thyllore_vulkan_core::types::Instance;

pub fn collect_mesh_transforms(world: &World, assets: &AssetStorage) -> Vec<Mat4> {
    let indexed_transforms: Vec<(usize, Mat4)> = world
        .iter_components::<MeshRef>()
        .filter_map(|(entity, mesh_ref)| {
            let mesh_asset = assets.get_mesh(mesh_ref.mesh_asset_id)?;
            let model_matrix = world
                .get_component::<GlobalTransform>(entity)
                .map(|global_transform| global_transform.0)
                .unwrap_or(Mat4::IDENTITY);
            Some((mesh_asset.graphics_mesh_index, model_matrix))
        })
        .collect();

    let transform_count = indexed_transforms
        .iter()
        .map(|(index, _)| index + 1)
        .max()
        .unwrap_or(0);

    let mut transforms = vec![Mat4::IDENTITY; transform_count];
    for (index, model_matrix) in indexed_transforms {
        transforms[index] = model_matrix;
    }

    transforms
}

pub unsafe fn refresh_tlas_mesh_transforms(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.raytracing.has_valid_tlas() {
        return Ok(());
    }

    let mesh_transforms = collect_mesh_transforms(ctx.world, ctx.assets);
    let procedural_primitives = crate::hooks::gpu_primitive::collect_all(ctx.world);
    let gbuffer_mesh_indices: Vec<usize> = ctx
        .graphics
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, mesh)| mesh.render_to_gbuffer)
        .map(|(mesh_index, _)| mesh_index)
        .collect();

    let Some(acceleration_structure) = ctx.raytracing.acceleration_structure.as_mut() else {
        return Ok(());
    };

    if acceleration_structure.blas_list.len() != gbuffer_mesh_indices.len()
        || acceleration_structure.procedural_blas.len() != procedural_primitives.len()
    {
        return Ok(());
    }

    let mut needs_update = false;
    for (blas, &mesh_index) in acceleration_structure
        .blas_list
        .iter_mut()
        .zip(gbuffer_mesh_indices.iter())
    {
        let model = mesh_transforms
            .get(mesh_index)
            .copied()
            .unwrap_or(Mat4::IDENTITY);
        needs_update |= apply_instance_transform(blas, &model);
    }
    for (blas, primitive) in acceleration_structure
        .procedural_blas
        .iter_mut()
        .zip(procedural_primitives.iter())
    {
        needs_update |= apply_instance_transform(blas, &primitive.model);
    }

    if !needs_update {
        return Ok(());
    }

    RRAccelerationStructure::update_tlas(
        ctx.instance,
        ctx.device,
        ctx.command_pool.as_ref(),
        &mut acceleration_structure.tlas,
        &acceleration_structure.blas_list,
        &acceleration_structure.procedural_blas,
    )
}

pub unsafe fn rebuild_scene_acceleration(
    gpu: &mut GpuSceneContext,
    world: &World,
    assets: &AssetStorage,
) -> Result<()> {
    let procedural_primitives = crate::hooks::gpu_primitive::collect_all(world);
    let mesh_transforms = collect_mesh_transforms(world, assets);
    gpu.raytracing.build_acceleration_structures(
        gpu.instance,
        gpu.device,
        gpu.command_pool,
        &gpu.graphics.meshes,
        &mesh_transforms,
        &procedural_primitives,
    )?;
    gpu.raytracing.bind_ray_query_tlas(gpu.device)
}

fn apply_instance_transform(blas: &mut RRBLAS, model: &Mat4) -> bool {
    let matrix = [
        [
            model.col(0)[0],
            model.col(1)[0],
            model.col(2)[0],
            model.col(3)[0],
        ],
        [
            model.col(0)[1],
            model.col(1)[1],
            model.col(2)[1],
            model.col(3)[1],
        ],
        [
            model.col(0)[2],
            model.col(1)[2],
            model.col(2)[2],
            model.col(3)[2],
        ],
    ];
    if blas.transform().matrix() == matrix {
        return false;
    }
    blas.set_transform_rows(matrix);
    true
}

/// traceRayEXT nesting the effect trace shaders use: the ray generation stage plus one secondary
/// trace from an effect's closest hit shader.
pub const EFFECT_TRACE_RECURSION_DEPTH: u32 = 2;

/// Creates the shared trace pipeline once. A device whose ray recursion limit is below what the
/// shaders need gets no pipeline, and every effect then renders without the trace pass.
pub unsafe fn ensure_effect_trace_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    world: &mut World,
    frames_in_flight: usize,
) -> Result<()> {
    if world
        .get_resource::<EffectTraceGpuState>()
        .is_some_and(|state| state.pipeline.is_some())
    {
        return Ok(());
    }

    let supported_depth = max_ray_recursion_depth(instance, rrdevice);
    if supported_depth < EFFECT_TRACE_RECURSION_DEPTH {
        log_warn!(
            "Effect trace pipeline skipped: device ray recursion depth {} < {}",
            supported_depth,
            EFFECT_TRACE_RECURSION_DEPTH
        );
        return Ok(());
    }

    let effect_trace_descriptor = RREffectTraceDescriptorSet::new(rrdevice, frames_in_flight)?;
    let effect_trace_pipeline = RRRayTracingPipeline::new(
        instance,
        rrdevice,
        &EFFECT_TRACE,
        &[effect_trace_descriptor.layout.handle()],
        &[push_constant_range(&effect_trace::PUSH_CONSTANT)],
        EFFECT_TRACE_RECURSION_DEPTH,
    )?;

    world.insert_resource(EffectTraceGpuState {
        pipeline: Some(effect_trace_pipeline),
        descriptor: Some(effect_trace_descriptor),
    });

    log!("Created effect trace pipeline");
    Ok(())
}
