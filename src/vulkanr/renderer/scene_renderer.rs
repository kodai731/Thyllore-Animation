use anyhow::Result;

use crate::ecs::component::RenderData;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::ObjectDescriptorSet;
use crate::vulkanr::resource::{GpuBufferRegistry, PipelineStorage};
use thyllore_render_core::ObjectUBO;
use thyllore_vulkan_core::renderer::{
    bind_graphics_descriptor_set, bind_graphics_mesh, draw_indexed, IndexWidth,
};
use thyllore_vulkan_core::types::{CommandBuffer, DescriptorSet};

pub unsafe fn update_object_ubo(
    render_data: &[&RenderData],
    image_index: usize,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
) -> Result<()> {
    for data in render_data {
        let ubo = ObjectUBO {
            model: data.model_matrix,
        };
        objects.update(rrdevice, image_index, data.render_info.object_index, &ubo)?;
    }
    Ok(())
}

pub unsafe fn render_scene_objects(
    render_data: &[&RenderData],
    command_buffer: CommandBuffer,
    image_index: usize,
    frame_set: DescriptorSet,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
    pipeline_storage: &PipelineStorage,
    buffer_registry: &GpuBufferRegistry,
) {
    for data in render_data {
        let vertex_buffer =
            match buffer_registry.get_vertex_buffer(data.mesh_ref.vertex_buffer_handle) {
                Some(b) => b,
                None => continue,
            };
        let index_buffer = match buffer_registry.get_index_buffer(data.mesh_ref.index_buffer_handle)
        {
            Some(b) => b,
            None => continue,
        };

        let pipeline_id = match data.render_info.pipeline_id {
            Some(id) => id,
            None => continue,
        };
        let pipeline = match pipeline_storage.get(pipeline_id) {
            Some(p) => p,
            None => continue,
        };

        bind_graphics_mesh(
            rrdevice,
            command_buffer,
            pipeline.pipeline,
            vertex_buffer,
            index_buffer,
            IndexWidth::U32,
        );

        bind_graphics_descriptor_set(
            rrdevice,
            command_buffer,
            pipeline.pipeline_layout,
            0,
            frame_set,
        );

        let object_set_idx = objects.get_set_index(image_index, data.render_info.object_index);
        let object_set = objects.sets[object_set_idx];
        bind_graphics_descriptor_set(
            rrdevice,
            command_buffer,
            pipeline.pipeline_layout,
            2,
            object_set,
        );

        draw_indexed(rrdevice, command_buffer, data.mesh_ref.index_count, 0, 0);
    }
}
