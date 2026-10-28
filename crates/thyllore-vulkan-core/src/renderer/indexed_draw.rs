use crate::core::device::RRDevice;
use crate::types::*;
use crate::vulkan::*;

pub enum IndexWidth {
    U16,
    U32,
}

impl IndexWidth {
    fn index_type(self) -> vk::IndexType {
        match self {
            IndexWidth::U16 => vk::IndexType::UINT16,
            IndexWidth::U32 => vk::IndexType::UINT32,
        }
    }
}

pub unsafe fn bind_graphics_mesh(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    pipeline: Pipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_width: IndexWidth,
) {
    let command_buffer = command_buffer.raw();
    rrdevice.device().cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.raw(),
    );

    rrdevice
        .device()
        .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer.raw()], &[0]);

    rrdevice.device().cmd_bind_index_buffer(
        command_buffer,
        index_buffer.raw(),
        0,
        index_width.index_type(),
    );
}

pub unsafe fn bind_graphics_descriptor_set(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    pipeline_layout: PipelineLayout,
    set_index: u32,
    descriptor_set: DescriptorSet,
) {
    let command_buffer = command_buffer.raw();
    rrdevice.device().cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline_layout.raw(),
        set_index,
        &[descriptor_set.raw()],
        &[],
    );
}

pub unsafe fn draw_indexed(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    index_count: u32,
    first_index: u32,
    vertex_offset: i32,
) {
    let command_buffer = command_buffer.raw();
    rrdevice.device().cmd_draw_indexed(
        command_buffer,
        index_count,
        1,
        first_index,
        vertex_offset,
        0,
    );
}

pub unsafe fn set_viewport_size(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    width: f32,
    height: f32,
) {
    let command_buffer = command_buffer.raw();
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width,
        height,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    rrdevice
        .device()
        .cmd_set_viewport(command_buffer, 0, &[viewport]);
}

pub unsafe fn set_scissor(rrdevice: &RRDevice, command_buffer: CommandBuffer, scissor: Rect2D) {
    let command_buffer = command_buffer.raw();
    rrdevice
        .device()
        .cmd_set_scissor(command_buffer, 0, &[scissor.raw()]);
}

pub unsafe fn push_vertex_constants(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    pipeline_layout: PipelineLayout,
    bytes: &[u8],
) {
    let command_buffer = command_buffer.raw();
    rrdevice.device().cmd_push_constants(
        command_buffer,
        pipeline_layout.raw(),
        vk::ShaderStageFlags::VERTEX,
        0,
        bytes,
    );
}
