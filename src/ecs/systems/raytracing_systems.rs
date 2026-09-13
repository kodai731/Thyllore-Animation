use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix};
use thyllore_vulkan_core::core::RRDevice;
use thyllore_vulkan_core::descriptor::{RREffectTraceDescriptorSet, EFFECT_TRACE};
use thyllore_vulkan_core::pipeline::RRRayTracingPipeline;
use thyllore_vulkan_core::raytracing::{RRAccelerationStructure, RRBLAS};
use thyllore_vulkan_core::resource::RayTracingData;
use vulkanalia::prelude::v1_0::*;

use crate::app::FrameContext;
use crate::asset::AssetStorage;
use crate::ecs::world::{GlobalTransform, MeshRef, World};

pub fn collect_mesh_transforms(world: &World, assets: &AssetStorage) -> Vec<Matrix4<f32>> {
    let indexed_transforms: Vec<(usize, Matrix4<f32>)> = world
        .iter_components::<MeshRef>()
        .filter_map(|(entity, mesh_ref)| {
            let mesh_asset = assets.get_mesh(mesh_ref.mesh_asset_id)?;
            let model_matrix = world
                .get_component::<GlobalTransform>(entity)
                .map(|global_transform| global_transform.0)
                .unwrap_or_else(Matrix4::identity);
            Some((mesh_asset.graphics_mesh_index, model_matrix))
        })
        .collect();

    let transform_count = indexed_transforms
        .iter()
        .map(|(index, _)| index + 1)
        .max()
        .unwrap_or(0);

    let mut transforms = vec![Matrix4::identity(); transform_count];
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
            .unwrap_or_else(Matrix4::identity);
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

fn apply_instance_transform(blas: &mut RRBLAS, model: &Matrix4<f32>) -> bool {
    let matrix = [
        [model[0][0], model[1][0], model[2][0], model[3][0]],
        [model[0][1], model[1][1], model[2][1], model[3][1]],
        [model[0][2], model[1][2], model[2][2], model[3][2]],
    ];
    if blas.transform.matrix == matrix {
        return false;
    }
    blas.transform.matrix = matrix;
    true
}

pub unsafe fn ensure_effect_trace_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    raytracing: &mut RayTracingData,
    frames_in_flight: usize,
) -> Result<()> {
    if raytracing.effect_trace_pipeline.is_some() {
        return Ok(());
    }

    let effect_trace_descriptor = RREffectTraceDescriptorSet::new(rrdevice, frames_in_flight)?;
    let effect_trace_pipeline = RRRayTracingPipeline::new(
        instance,
        rrdevice,
        &EFFECT_TRACE,
        &[effect_trace_descriptor.layout.handle],
        &trace_push_constant_ranges(),
    )?;

    raytracing.effect_trace_descriptor = Some(effect_trace_descriptor);
    raytracing.effect_trace_pipeline = Some(effect_trace_pipeline);

    log!("Created effect trace pipeline");
    Ok(())
}

fn trace_push_constant_ranges() -> [vk::PushConstantRange; 3] {
    let intersection_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::INTERSECTION_KHR)
        .offset(0)
        .size(8)
        .build();
    let raygen_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
        .offset(16)
        .size(112)
        .build();
    let closest_hit_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
        .offset(96)
        .size(32)
        .build();
    [intersection_range, raygen_range, closest_hit_range]
}
