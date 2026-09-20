use std::ffi::CStr;

use crate::ecs::resource::{WaterDebugCapture, WaterRenderTargets};
use crate::ecs::systems::{
    build_water_debug_record, current_unix_time, water_debug_caustic_accum_path,
    water_debug_screenshot_path, write_water_debug_dump, WaterDebugRenderInfo,
};
use crate::hooks::batch_capture::{BatchCapture, CaptureContext};
use crate::vulkanr::context::SwapchainState;
use crate::vulkanr::data::Vertex;
use crate::vulkanr::vulkan::*;
use thyllore_math_core::write_npy_u32;

use serde_json::{json, Value};
use thyllore_vulkan_core::raytracing::RRAccelerationStructure;
use thyllore_vulkan_core::resource::mesh_buffer::MeshBuffer;

const VERTEX_PROBE_COUNT: usize = 64;

#[derive(Clone, Copy, Debug, Default)]
pub struct WaterCausticAccumStats {
    pub nonzero_count: u64,
    pub max_value: u32,
}

unsafe fn save_water_caustic_accum_npy(
    ctx: &CaptureContext,
    path: &std::path::Path,
) -> Result<WaterCausticAccumStats> {
    let device = &ctx.device.device;
    let water_targets = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .ok_or_else(|| anyhow::anyhow!("water buffer not initialized"))?;
    let caustic_image = water_targets.caustic_accum.image;
    let width = water_targets.history.width;
    let height = water_targets.history.height;
    let image_size = (width * height * 4) as vk::DeviceSize;

    let (buffer, buffer_memory) = ctx.copy_image_to_buffer(
        caustic_image,
        width,
        height,
        image_size,
        vk::ImageLayout::GENERAL,
    )?;

    let data_ptr = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
    let slice = std::slice::from_raw_parts(data_ptr as *const u8, image_size as usize);

    let mut u32_data: Vec<u32> = Vec::with_capacity((width * height) as usize);
    let mut stats = WaterCausticAccumStats::default();
    for chunk in slice.chunks_exact(4) {
        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if value != 0 {
            stats.nonzero_count += 1;
        }
        stats.max_value = stats.max_value.max(value);
        u32_data.push(value);
    }

    device.unmap_memory(buffer_memory);
    device.free_memory(buffer_memory, None);
    device.destroy_buffer(buffer, None);

    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    write_npy_u32(path, &[height as usize, width as usize], &u32_data)?;

    Ok(stats)
}

impl BatchCapture for WaterDebugCapture {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        dump_water_debug(ctx);
        Ok(())
    }
}

crate::batch_capture!(WaterDebugCapture);
crate::capture_action!("dump_water_debug", WaterDebugCapture);

pub fn dump_water_debug(ctx: &CaptureContext) {
    let unix_time = current_unix_time();
    let mut render_info = collect_water_debug_render_info(ctx);

    let screenshot_path = water_debug_screenshot_path(unix_time);
    match unsafe { ctx.save_screenshot_to(&screenshot_path) } {
        Ok(_) => render_info.screenshot_path = Some(screenshot_path.display().to_string()),
        Err(error) => log_warn!("water debug screenshot failed: {:?}", error),
    }

    let caustic_accum_path = water_debug_caustic_accum_path(unix_time);
    match unsafe { save_water_caustic_accum_npy(ctx, &caustic_accum_path) } {
        Ok(stats) => {
            render_info.caustic_accum_path = Some(caustic_accum_path.display().to_string());
            render_info.caustic_accum_nonzero = Some(stats.nonzero_count);
            render_info.caustic_accum_max = Some(stats.max_value);
        }
        Err(error) => log_warn!("water debug caustic accum save failed: {:?}", error),
    }

    let record = build_water_debug_record(ctx.world, &render_info, unix_time);
    match write_water_debug_dump(&record, unix_time) {
        Ok(path) => msg_info!("Water debug dumped: {}", path.display()),
        Err(error) => log_error!("water debug dump failed: {}", error),
    }
}

fn collect_water_debug_render_info(ctx: &CaptureContext) -> WaterDebugRenderInfo {
    let properties = unsafe {
        ctx.instance
            .get_physical_device_properties(ctx.device.physical_device)
    };
    let gpu_name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) }
        .to_string_lossy()
        .to_string();

    let swapchain_extent = ctx
        .world
        .resource::<SwapchainState>()
        .swapchain
        .swapchain_extent;
    let water_buffer_size = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .map(|targets| [targets.history.width, targets.history.height]);
    let acceleration = ctx.raytracing.acceleration_structure.as_ref();

    WaterDebugRenderInfo {
        gpu_name,
        driver_version: format_vulkan_version(properties.driver_version),
        api_version: format_vulkan_version(properties.api_version),
        swapchain_size: [swapchain_extent.width, swapchain_extent.height],
        hdr_buffer_size: ctx.hdr.map(|b| [b.width, b.height]),
        water_buffer_size,
        mesh_count: ctx.graphics.meshes.len(),
        mesh_blas_count: acceleration.map(|a| a.blas_list.len()).unwrap_or(0),
        procedural_blas_count: acceleration.map(|a| a.procedural_blas.len()).unwrap_or(0),
        hit_shading_table_capacity: acceleration
            .and_then(|a| a.hit_shading_table.as_ref())
            .map(|table| table.capacity),
        screenshot_path: None,
        caustic_accum_path: None,
        caustic_accum_nonzero: None,
        caustic_accum_max: None,
        tlas_instances: acceleration.map(|a| build_tlas_instances_json(ctx, a)),
        mesh_vertex_probe: acceleration.map(|_| build_mesh_vertex_probe_json(ctx)),
    }
}

fn build_tlas_instances_json(
    ctx: &CaptureContext,
    acceleration: &RRAccelerationStructure,
) -> Value {
    let mut instances: Vec<Value> = acceleration
        .blas_list
        .iter()
        .enumerate()
        .zip(collect_gbuffer_mesh_indices(ctx))
        .map(|((blas_index, blas), mesh_index)| {
            let mesh = &ctx.graphics.meshes[mesh_index];
            json!({
                "blas_index": blas_index,
                "mesh_index": mesh_index,
                "transform": blas.transform.matrix,
                "vertex_count": mesh.vertex_data.vertices.len(),
                "index_count": mesh.vertex_data.indices.len(),
            })
        })
        .collect();

    let blas_count = acceleration.blas_list.len();
    instances.extend(
        acceleration
            .procedural_blas
            .iter()
            .enumerate()
            .map(|(water_index, blas)| {
                json!({
                    "blas_index": blas_count + water_index,
                    "kind": "water",
                    "transform": blas.transform.matrix,
                })
            }),
    );

    Value::Array(instances)
}

fn build_mesh_vertex_probe_json(ctx: &CaptureContext) -> Value {
    let probes: Vec<Value> = collect_gbuffer_mesh_indices(ctx)
        .into_iter()
        .map(|mesh_index| {
            let mesh = &ctx.graphics.meshes[mesh_index];
            let sample_count = mesh.vertex_data.vertices.len().min(VERTEX_PROBE_COUNT);
            let cpu_positions: Vec<[f32; 3]> = mesh.vertex_data.vertices[..sample_count]
                .iter()
                .map(|vertex| [vertex.pos[0], vertex.pos[1], vertex.pos[2]])
                .collect();
            let gpu_positions = match unsafe { read_gpu_vertex_positions(ctx, mesh, sample_count) }
            {
                Ok(positions) => positions,
                Err(error) => {
                    log_warn!("water debug vertex probe failed: {:?}", error);
                    Vec::new()
                }
            };

            json!({
                "mesh_index": mesh_index,
                "sample_count": sample_count,
                "gpu_pos_first": gpu_positions.first(),
                "gpu_pos_centroid64": average_position(&gpu_positions),
                "cpu_pos_first": cpu_positions.first(),
                "cpu_pos_centroid64": average_position(&cpu_positions),
            })
        })
        .collect();

    Value::Array(probes)
}

unsafe fn read_gpu_vertex_positions(
    ctx: &CaptureContext,
    mesh: &MeshBuffer,
    sample_count: usize,
) -> Result<Vec<[f32; 3]>> {
    if sample_count == 0 {
        return Ok(Vec::new());
    }

    let device = &ctx.device.device;
    let stride = std::mem::size_of::<Vertex>();
    let copy_size = (stride * sample_count) as vk::DeviceSize;
    let command_pool = ctx.command_pool;

    let (buffer, buffer_memory) = allocate_vertex_probe_buffer(ctx, copy_size)?;
    let command_buffer = record_and_submit_vertex_copy(
        ctx,
        command_pool,
        mesh.vertex_buffer.buffer,
        buffer,
        copy_size,
    )?;

    let mapped = device.map_memory(buffer_memory, 0, copy_size, vk::MemoryMapFlags::empty())?;
    let bytes = std::slice::from_raw_parts(mapped as *const u8, copy_size as usize);
    let pos_offset = std::mem::offset_of!(Vertex, pos);
    let positions = (0..sample_count)
        .map(|index| read_position(bytes, index * stride + pos_offset))
        .collect();

    device.unmap_memory(buffer_memory);
    device.free_command_buffers(command_pool, &[command_buffer]);
    device.free_memory(buffer_memory, None);
    device.destroy_buffer(buffer, None);

    Ok(positions)
}

unsafe fn allocate_vertex_probe_buffer(
    ctx: &CaptureContext,
    size: vk::DeviceSize,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let device = &ctx.device.device;

    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(vk::BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = device.create_buffer(&buffer_info, None)?;

    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_type_index = thyllore_vulkan_core::vulkan::get_memory_type_index(
        ctx.instance,
        ctx.device.physical_device,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        requirements,
    )?;
    let allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index);
    let buffer_memory = device.allocate_memory(&allocate_info, None)?;
    device.bind_buffer_memory(buffer, buffer_memory, 0)?;

    Ok((buffer, buffer_memory))
}

unsafe fn record_and_submit_vertex_copy(
    ctx: &CaptureContext,
    command_pool: vk::CommandPool,
    source: vk::Buffer,
    destination: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<vk::CommandBuffer> {
    let device = &ctx.device.device;

    let allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let command_buffer = device.allocate_command_buffers(&allocate_info)?[0];

    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device.begin_command_buffer(command_buffer, &begin_info)?;
    let region = vk::BufferCopy::builder()
        .src_offset(0)
        .dst_offset(0)
        .size(size);
    device.cmd_copy_buffer(command_buffer, source, destination, &[region]);
    device.end_command_buffer(command_buffer)?;

    let command_buffers = [command_buffer];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffers);
    device.queue_submit(
        ctx.device.graphics_queue,
        &[submit_info.build()],
        vk::Fence::null(),
    )?;
    device.queue_wait_idle(ctx.device.graphics_queue)?;

    Ok(command_buffer)
}

fn collect_gbuffer_mesh_indices(ctx: &CaptureContext) -> Vec<usize> {
    ctx.graphics
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, mesh)| mesh.render_to_gbuffer)
        .map(|(mesh_index, _)| mesh_index)
        .collect()
}

fn read_position(bytes: &[u8], offset: usize) -> [f32; 3] {
    let read_float = |component: usize| {
        let start = offset + component * std::mem::size_of::<f32>();
        let mut raw = [0u8; 4];
        raw.copy_from_slice(&bytes[start..start + 4]);
        f32::from_le_bytes(raw)
    };
    [read_float(0), read_float(1), read_float(2)]
}

fn average_position(positions: &[[f32; 3]]) -> Option<[f32; 3]> {
    if positions.is_empty() {
        return None;
    }
    let inverse_count = 1.0 / positions.len() as f32;
    let sum = positions.iter().fold([0.0f32; 3], |mut sum, position| {
        sum[0] += position[0];
        sum[1] += position[1];
        sum[2] += position[2];
        sum
    });
    Some([
        sum[0] * inverse_count,
        sum[1] * inverse_count,
        sum[2] * inverse_count,
    ])
}

fn format_vulkan_version(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::version_major(version),
        vk::version_minor(version),
        vk::version_patch(version)
    )
}
