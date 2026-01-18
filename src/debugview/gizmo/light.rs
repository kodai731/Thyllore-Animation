use super::grid::GizmoVertex;
use crate::ecs::RenderData;
use crate::log;
use crate::math::{Vec2, Vec3, Vec4};
use crate::vulkanr::buffer::*;
use crate::vulkanr::command::*;
use crate::vulkanr::core::Device;
use crate::vulkanr::data::Vertex;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::vulkan::*;
use cgmath::{InnerSpace, Matrix4, Vector3};
use std::mem::size_of;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LightGizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
    Center,
}

#[derive(Clone, Debug)]
pub struct LightGizmoData {
    pub pipeline: RRPipeline,
    pub object_index: usize,
    pub position: Vector3<f32>,
    pub vertices: Vec<GizmoVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub selected_axis: LightGizmoAxis,
    pub is_selected: bool,
    pub drag_axis: LightGizmoAxis,
    pub just_selected: bool,
    pub initial_position: [f32; 3],
    pub ray_to_model_vertices: Vec<Vertex>,
    pub ray_to_model_indices: Vec<u32>,
    pub ray_to_model_vertex_buffer: Option<vk::Buffer>,
    pub ray_to_model_vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub ray_to_model_index_buffer: Option<vk::Buffer>,
    pub ray_to_model_index_buffer_memory: Option<vk::DeviceMemory>,
    pub vertical_line_vertices: Vec<Vertex>,
    pub vertical_line_indices: Vec<u32>,
    pub vertical_line_vertex_buffer: Option<vk::Buffer>,
    pub vertical_line_vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub vertical_line_index_buffer: Option<vk::Buffer>,
    pub vertical_line_index_buffer_memory: Option<vk::DeviceMemory>,
}

impl Default for LightGizmoData {
    fn default() -> Self {
        Self::new(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl LightGizmoData {
    pub fn new(position: Vector3<f32>) -> Self {
        let axis_length = 1.0;
        let yellow = [1.0, 1.0, 0.0];

        let vertices = vec![
            GizmoVertex {
                pos: [0.0, 0.0, 0.0],
                color: yellow,
            },
            GizmoVertex {
                pos: [axis_length, 0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            GizmoVertex {
                pos: [0.0, axis_length, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            GizmoVertex {
                pos: [0.0, 0.0, axis_length],
                color: [0.0, 0.0, 1.0],
            },
        ];

        let indices = vec![0, 1, 0, 2, 0, 3];

        Self {
            pipeline: RRPipeline::default(),
            object_index: 0,
            position,
            vertices,
            indices,
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
            selected_axis: LightGizmoAxis::None,
            is_selected: false,
            drag_axis: LightGizmoAxis::None,
            just_selected: false,
            initial_position: [0.0, 0.0, 0.0],
            ray_to_model_vertices: Vec::new(),
            ray_to_model_indices: Vec::new(),
            ray_to_model_vertex_buffer: None,
            ray_to_model_vertex_buffer_memory: None,
            ray_to_model_index_buffer: None,
            ray_to_model_index_buffer_memory: None,
            vertical_line_vertices: Vec::new(),
            vertical_line_indices: Vec::new(),
            vertical_line_vertex_buffer: None,
            vertical_line_vertex_buffer_memory: None,
            vertical_line_index_buffer: None,
            vertical_line_index_buffer_memory: None,
        }
    }

    pub unsafe fn create_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vertex_buffer_size = (size_of::<GizmoVertex>() * self.vertices.len()) as u64;
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice.device.map_memory(
            vertex_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(self.vertices.as_ptr(), data.cast(), self.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);

        self.vertex_buffer = Some(vertex_buffer);
        self.vertex_buffer_memory = Some(vertex_buffer_memory);

        let index_buffer_size = (size_of::<u32>() * self.indices.len()) as u64;
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice.device.map_memory(
            staging_buffer_memory,
            0,
            index_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(self.indices.as_ptr(), data.cast(), self.indices.len());
        rrdevice.device.unmap_memory(staging_buffer_memory);

        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            rrdevice,
            rrcommand_pool,
            staging_buffer,
            index_buffer,
            index_buffer_size,
        )?;

        rrdevice.device.destroy_buffer(staging_buffer, None);
        rrdevice.device.free_memory(staging_buffer_memory, None);

        self.index_buffer = Some(index_buffer);
        self.index_buffer_memory = Some(index_buffer_memory);

        Ok(())
    }

    pub unsafe fn update_vertex_buffer(
        &self,
        rrdevice: &RRDevice,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(vertex_buffer_memory) = self.vertex_buffer_memory {
            let vertex_buffer_size = (size_of::<GizmoVertex>() * self.vertices.len()) as u64;
            let data = rrdevice.device.map_memory(
                vertex_buffer_memory,
                0,
                vertex_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(self.vertices.as_ptr(), data.cast(), self.vertices.len());
            rrdevice.device.unmap_memory(vertex_buffer_memory);
        }
        Ok(())
    }

    pub fn update_ray_to_model(&mut self, model_positions: &[Vector3<f32>]) {
        if model_positions.is_empty() {
            self.ray_to_model_vertices.clear();
            self.ray_to_model_indices.clear();
            return;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        let mut closest_point = model_positions[0];
        let mut closest_index: usize = 0;
        let mut min_distance = (closest_point - self.position).magnitude();

        for (i, pos) in model_positions.iter().enumerate() {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
            min_z = min_z.min(pos.z);
            max_z = max_z.max(pos.z);

            let distance = (*pos - self.position).magnitude();
            if distance < min_distance {
                min_distance = distance;
                closest_point = *pos;
                closest_index = i;
            }
        }

        let bright_yellow = Vec4::new(1.0, 1.0, 0.0, 1.0);
        let tex_coord = Vec2::new(0.0, 0.0);

        let light_pos = Vec3::new(self.position.x, self.position.y, self.position.z);
        let closest = Vec3::new(closest_point.x, closest_point.y, closest_point.z);

        let vertex_0 = Vertex::new(light_pos, bright_yellow, tex_coord);
        let vertex_1 = Vertex::new(closest, bright_yellow, tex_coord);

        self.ray_to_model_vertices = vec![vertex_0, vertex_1];
        self.ray_to_model_indices = vec![0, 1];

        static mut VERTEX_LOG_COUNTER: u32 = 0;
        unsafe {
            VERTEX_LOG_COUNTER += 1;
            if VERTEX_LOG_COUNTER % 120 == 1 {
                log!("=== Ray to Model Debug ===");
                log!("Model vertex count: {}", model_positions.len());
                log!(
                    "Model bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_z,
                    max_z
                );
                log!(
                    "Light position: ({:.2}, {:.2}, {:.2})",
                    self.position.x,
                    self.position.y,
                    self.position.z
                );
                log!("Closest vertex index: {}", closest_index);
                log!(
                    "Closest vertex position: ({:.2}, {:.2}, {:.2})",
                    closest_point.x,
                    closest_point.y,
                    closest_point.z
                );
                log!("Distance to closest: {:.2}", min_distance);
                log!(
                    "Ray line: [0]=Light({:.2}, {:.2}, {:.2}) -> [1]=Model({:.2}, {:.2}, {:.2})",
                    vertex_0.pos.x,
                    vertex_0.pos.y,
                    vertex_0.pos.z,
                    vertex_1.pos.x,
                    vertex_1.pos.y,
                    vertex_1.pos.z
                );
                log!("==========================");
            }
        }
    }

    pub unsafe fn update_or_create_ray_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
    ) -> anyhow::Result<()> {
        if self.ray_to_model_vertices.is_empty() {
            return Ok(());
        }

        let vertex_buffer_size = (size_of::<Vertex>() * self.ray_to_model_vertices.len()) as u64;

        if self.ray_to_model_vertex_buffer.is_none() {
            let (vertex_buffer, vertex_buffer_memory) = create_buffer(
                instance,
                rrdevice,
                vertex_buffer_size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.ray_to_model_vertex_buffer = Some(vertex_buffer);
            self.ray_to_model_vertex_buffer_memory = Some(vertex_buffer_memory);
        }

        if let (Some(vertex_buffer_memory), Some(_vertex_buffer)) = (
            self.ray_to_model_vertex_buffer_memory,
            self.ray_to_model_vertex_buffer,
        ) {
            let data = rrdevice.device.map_memory(
                vertex_buffer_memory,
                0,
                vertex_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                self.ray_to_model_vertices.as_ptr(),
                data.cast(),
                self.ray_to_model_vertices.len(),
            );
            rrdevice.device.unmap_memory(vertex_buffer_memory);
        }

        let index_buffer_size = (size_of::<u32>() * self.ray_to_model_indices.len()) as u64;

        if self.ray_to_model_index_buffer.is_none() {
            let (index_buffer, index_buffer_memory) = create_buffer(
                instance,
                rrdevice,
                index_buffer_size,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.ray_to_model_index_buffer = Some(index_buffer);
            self.ray_to_model_index_buffer_memory = Some(index_buffer_memory);
        }

        if let (Some(index_buffer_memory), Some(_index_buffer)) = (
            self.ray_to_model_index_buffer_memory,
            self.ray_to_model_index_buffer,
        ) {
            let data = rrdevice.device.map_memory(
                index_buffer_memory,
                0,
                index_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                self.ray_to_model_indices.as_ptr(),
                data.cast(),
                self.ray_to_model_indices.len(),
            );
            rrdevice.device.unmap_memory(index_buffer_memory);
        }

        Ok(())
    }

    pub unsafe fn destroy_buffers(&mut self, rrdevice: &RRDevice) {
        if let Some(vertex_buffer) = self.vertex_buffer {
            rrdevice.device.destroy_buffer(vertex_buffer, None);
        }
        if let Some(vertex_buffer_memory) = self.vertex_buffer_memory {
            rrdevice.device.free_memory(vertex_buffer_memory, None);
        }
        if let Some(index_buffer) = self.index_buffer {
            rrdevice.device.destroy_buffer(index_buffer, None);
        }
        if let Some(index_buffer_memory) = self.index_buffer_memory {
            rrdevice.device.free_memory(index_buffer_memory, None);
        }

        self.vertex_buffer = None;
        self.vertex_buffer_memory = None;
        self.index_buffer = None;
        self.index_buffer_memory = None;

        if let Some(ray_vertex_buffer) = self.ray_to_model_vertex_buffer {
            rrdevice.device.destroy_buffer(ray_vertex_buffer, None);
        }
        if let Some(ray_vertex_buffer_memory) = self.ray_to_model_vertex_buffer_memory {
            rrdevice.device.free_memory(ray_vertex_buffer_memory, None);
        }
        if let Some(ray_index_buffer) = self.ray_to_model_index_buffer {
            rrdevice.device.destroy_buffer(ray_index_buffer, None);
        }
        if let Some(ray_index_buffer_memory) = self.ray_to_model_index_buffer_memory {
            rrdevice.device.free_memory(ray_index_buffer_memory, None);
        }

        self.ray_to_model_vertex_buffer = None;
        self.ray_to_model_vertex_buffer_memory = None;
        self.ray_to_model_index_buffer = None;
        self.ray_to_model_index_buffer_memory = None;

        if let Some(buffer) = self.vertical_line_vertex_buffer {
            rrdevice.device.destroy_buffer(buffer, None);
        }
        if let Some(memory) = self.vertical_line_vertex_buffer_memory {
            rrdevice.device.free_memory(memory, None);
        }
        if let Some(buffer) = self.vertical_line_index_buffer {
            rrdevice.device.destroy_buffer(buffer, None);
        }
        if let Some(memory) = self.vertical_line_index_buffer_memory {
            rrdevice.device.free_memory(memory, None);
        }

        self.vertical_line_vertex_buffer = None;
        self.vertical_line_vertex_buffer_memory = None;
        self.vertical_line_index_buffer = None;
        self.vertical_line_index_buffer_memory = None;
    }

    pub fn update_vertical_lines(&mut self, model_positions: &[Vector3<f32>]) {
        let orange = Vec4::new(1.0, 0.5, 0.0, 1.0);
        let tex_coord = Vec2::new(0.0, 0.0);

        self.vertical_line_vertices.clear();
        self.vertical_line_indices.clear();

        let light_pos = Vec3::new(self.position.x, self.position.y, self.position.z);
        let light_ground = Vec3::new(self.position.x, 0.0, self.position.z);

        self.vertical_line_vertices
            .push(Vertex::new(light_pos, orange, tex_coord));
        self.vertical_line_vertices
            .push(Vertex::new(light_ground, orange, tex_coord));
        self.vertical_line_indices.push(0);
        self.vertical_line_indices.push(1);

        for (i, pos) in model_positions.iter().enumerate() {
            let top = Vec3::new(pos.x, pos.y, pos.z);
            let bottom = Vec3::new(pos.x, 0.0, pos.z);

            let base_index = (2 + i * 2) as u32;
            self.vertical_line_vertices
                .push(Vertex::new(top, orange, tex_coord));
            self.vertical_line_vertices
                .push(Vertex::new(bottom, orange, tex_coord));
            self.vertical_line_indices.push(base_index);
            self.vertical_line_indices.push(base_index + 1);
        }

        static mut LOG_COUNTER: u32 = 0;
        unsafe {
            LOG_COUNTER += 1;
            if LOG_COUNTER % 60 == 1 {
                log!(
                    "Vertical lines: light=({:.1},{:.1},{:.1}), models={}, vertices={}, indices={}",
                    light_pos.x,
                    light_pos.y,
                    light_pos.z,
                    model_positions.len(),
                    self.vertical_line_vertices.len(),
                    self.vertical_line_indices.len()
                );
                for (i, pos) in model_positions.iter().enumerate() {
                    log!(
                        "  Model[{}] top: ({:.1},{:.1},{:.1})",
                        i,
                        pos.x,
                        pos.y,
                        pos.z
                    );
                }
            }
        }
    }

    pub unsafe fn update_or_create_vertical_line_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
    ) -> anyhow::Result<()> {
        if self.vertical_line_vertices.is_empty() {
            return Ok(());
        }

        let vertex_buffer_size = (size_of::<Vertex>() * self.vertical_line_vertices.len()) as u64;

        if self.vertical_line_vertex_buffer.is_none() {
            let (vertex_buffer, vertex_buffer_memory) = create_buffer(
                instance,
                rrdevice,
                vertex_buffer_size.max(1024),
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.vertical_line_vertex_buffer = Some(vertex_buffer);
            self.vertical_line_vertex_buffer_memory = Some(vertex_buffer_memory);
        }

        if let Some(vertex_buffer_memory) = self.vertical_line_vertex_buffer_memory {
            let data = rrdevice.device.map_memory(
                vertex_buffer_memory,
                0,
                vertex_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                self.vertical_line_vertices.as_ptr(),
                data.cast(),
                self.vertical_line_vertices.len(),
            );
            rrdevice.device.unmap_memory(vertex_buffer_memory);
        }

        let index_buffer_size = (size_of::<u32>() * self.vertical_line_indices.len()) as u64;

        if self.vertical_line_index_buffer.is_none() {
            let (index_buffer, index_buffer_memory) = create_buffer(
                instance,
                rrdevice,
                index_buffer_size.max(256),
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.vertical_line_index_buffer = Some(index_buffer);
            self.vertical_line_index_buffer_memory = Some(index_buffer_memory);
        }

        if let Some(index_buffer_memory) = self.vertical_line_index_buffer_memory {
            let data = rrdevice.device.map_memory(
                index_buffer_memory,
                0,
                index_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                self.vertical_line_indices.as_ptr(),
                data.cast(),
                self.vertical_line_indices.len(),
            );
            rrdevice.device.unmap_memory(index_buffer_memory);
        }

        Ok(())
    }

    pub unsafe fn draw_ray_to_model_with_resources(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        grid_pipeline: &RRPipeline,
        graphics_resources: &crate::scene::graphics_resource::GraphicsResources,
        object_index: usize,
        image_index: usize,
    ) {
        if let (Some(vertex_buffer), Some(index_buffer)) = (
            self.ray_to_model_vertex_buffer,
            self.ray_to_model_index_buffer,
        ) {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                grid_pipeline.pipeline,
            );

            device.cmd_set_line_width(command_buffer, 1.0);

            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

            device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

            let frame_set = graphics_resources.frame_set.sets[image_index];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                grid_pipeline.pipeline_layout,
                0,
                &[frame_set],
                &[],
            );

            let object_set_idx = graphics_resources.objects.get_set_index(image_index, object_index);
            let object_set = graphics_resources.objects.sets[object_set_idx];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                grid_pipeline.pipeline_layout,
                2,
                &[object_set],
                &[],
            );

            device.cmd_draw_indexed(
                command_buffer,
                self.ray_to_model_indices.len() as u32,
                1,
                0,
                0,
                0,
            );
        }
    }

    pub unsafe fn draw_vertical_lines_with_resources(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        grid_pipeline: &RRPipeline,
        graphics_resources: &crate::scene::graphics_resource::GraphicsResources,
        object_index: usize,
        image_index: usize,
    ) {
        if self.vertical_line_indices.is_empty() {
            return;
        }

        if self.vertical_line_vertex_buffer.is_none() || self.vertical_line_index_buffer.is_none() {
            return;
        }

        if let (Some(vertex_buffer), Some(index_buffer)) = (
            self.vertical_line_vertex_buffer,
            self.vertical_line_index_buffer,
        ) {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                grid_pipeline.pipeline,
            );

            device.cmd_set_line_width(command_buffer, 1.0);

            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

            device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

            let frame_set = graphics_resources.frame_set.sets[image_index];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                grid_pipeline.pipeline_layout,
                0,
                &[frame_set],
                &[],
            );

            let object_set_idx = graphics_resources.objects.get_set_index(image_index, object_index);
            let object_set = graphics_resources.objects.sets[object_set_idx];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                grid_pipeline.pipeline_layout,
                2,
                &[object_set],
                &[],
            );

            device.cmd_draw_indexed(
                command_buffer,
                self.vertical_line_indices.len() as u32,
                1,
                0,
                0,
                0,
            );
        }
    }

    pub fn render_data(&self, camera_position: Vector3<f32>) -> RenderData {
        let distance = (self.position - camera_position).magnitude();
        let scale_factor = distance * 0.03;
        let model_matrix = Matrix4::from_translation(self.position) * Matrix4::from_scale(scale_factor);

        RenderData {
            object_index: self.object_index,
            pipeline: self.pipeline.clone(),
            vertex_buffer: self.vertex_buffer.unwrap_or(vk::Buffer::null()),
            index_buffer: self.index_buffer.unwrap_or(vk::Buffer::null()),
            index_count: self.indices.len() as u32,
            model_matrix,
        }
    }
}
