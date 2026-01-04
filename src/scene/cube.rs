use crate::vulkanr::buffer::*;
use crate::vulkanr::core::swapchain::RRSwapchain;
use crate::vulkanr::data::{RRData, UniformBufferObject, Vertex};
use crate::vulkanr::descriptor::RRDescriptorSet;
use crate::vulkanr::device::*;
use crate::vulkanr::image::*;
use crate::vulkanr::vulkan::*;
use crate::vulkanr::command::RRCommandPool;
use crate::math::*;
use crate::log;

use anyhow::Result;
use std::mem::size_of;
use std::os::raw::c_void;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct CubeModel {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub rrdata: Option<RRData>,
    pub descriptor_set: Option<RRDescriptorSet>,
}

impl CubeModel {
    pub fn new(size: f32) -> Self {
        let half = size / 2.0;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let faces = [
            ([0.0, 0.0, 1.0], [
                [-half, -half,  half],
                [-half,  half,  half],
                [ half,  half,  half],
                [ half, -half,  half],
            ]),
            ([0.0, 0.0, -1.0], [
                [ half, -half, -half],
                [ half,  half, -half],
                [-half,  half, -half],
                [-half, -half, -half],
            ]),
            ([1.0, 0.0, 0.0], [
                [ half, -half,  half],
                [ half,  half,  half],
                [ half,  half, -half],
                [ half, -half, -half],
            ]),
            ([-1.0, 0.0, 0.0], [
                [-half, -half, -half],
                [-half,  half, -half],
                [-half,  half,  half],
                [-half, -half,  half],
            ]),
            ([0.0, 1.0, 0.0], [
                [-half,  half,  half],
                [-half,  half, -half],
                [ half,  half, -half],
                [ half,  half,  half],
            ]),
            ([0.0, -1.0, 0.0], [
                [-half, -half, -half],
                [-half, -half,  half],
                [ half, -half,  half],
                [ half, -half, -half],
            ]),
        ];

        let tex_coords = [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];

        let face_colors = [
            Vec4::new(1.0, 0.3, 0.3, 1.0),
            Vec4::new(0.3, 1.0, 0.3, 1.0),
            Vec4::new(0.3, 0.3, 1.0, 1.0),
            Vec4::new(1.0, 1.0, 0.3, 1.0),
            Vec4::new(0.3, 1.0, 1.0, 1.0),
            Vec4::new(1.0, 0.3, 1.0, 1.0),
        ];

        for (face_idx, (normal, positions)) in faces.iter().enumerate() {
            let base_index = vertices.len() as u32;
            let color = face_colors[face_idx];

            for (i, pos) in positions.iter().enumerate() {
                vertices.push(Vertex {
                    pos: Vec3::new(pos[0], pos[1], pos[2]),
                    color,
                    tex_coord: Vec2::new(tex_coords[i][0], tex_coords[i][1]),
                    normal: Vec3::new(normal[0], normal[1], normal[2]),
                });
            }

            indices.push(base_index);
            indices.push(base_index + 1);
            indices.push(base_index + 2);
            indices.push(base_index);
            indices.push(base_index + 2);
            indices.push(base_index + 3);
        }

        Self {
            vertices,
            indices,
            rrdata: None,
            descriptor_set: None,
        }
    }

    pub fn new_at_position(size: f32, position: [f32; 3]) -> Self {
        let mut cube = Self::new(size);
        for vertex in &mut cube.vertices {
            vertex.pos.x += position[0];
            vertex.pos.y += position[1];
            vertex.pos.z += position[2];
        }
        cube
    }

    pub unsafe fn initialize_gpu_resources(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &Rc<RRCommandPool>,
    ) -> Result<()> {
        let mut rrdata = RRData::new(instance, rrdevice, rrswapchain, "cube");

        (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
            instance,
            rrdevice,
            rrcommand_pool,
            &vec![255u8, 255, 255, 255],
            1,
            1,
        )?;

        rrdata.image_view = create_image_view(
            rrdevice,
            rrdata.image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
            rrdata.mip_level,
        )?;

        rrdata.sampler = create_texture_sampler(rrdevice, rrdata.mip_level)?;

        rrdata.vertex_data.vertices = self.vertices.clone();
        rrdata.vertex_data.indices = self.indices.clone();

        rrdata.vertex_buffer = RRVertexBuffer::new(
            instance,
            rrdevice,
            rrcommand_pool,
            (size_of::<Vertex>() * rrdata.vertex_data.vertices.len()) as vk::DeviceSize,
            rrdata.vertex_data.vertices.as_ptr() as *const c_void,
            rrdata.vertex_data.vertices.len(),
        );

        rrdata.index_buffer = RRIndexBuffer::new(
            instance,
            rrdevice,
            rrcommand_pool,
            (size_of::<u32>() * rrdata.vertex_data.indices.len()) as vk::DeviceSize,
            rrdata.vertex_data.indices.as_ptr() as *const c_void,
            rrdata.vertex_data.indices.len(),
        );

        let mut descriptor_set = RRDescriptorSet::new(rrdevice, rrswapchain);
        descriptor_set.rrdata.push(rrdata.clone());
        RRDescriptorSet::create_descriptor_set(rrdevice, rrswapchain, &mut descriptor_set)?;

        self.rrdata = Some(rrdata);
        self.descriptor_set = Some(descriptor_set);

        log!("CubeModel GPU resources initialized");
        Ok(())
    }

    pub unsafe fn update_uniform_buffer(
        &mut self,
        rrdevice: &RRDevice,
        image_index: usize,
        ubo: &UniformBufferObject,
    ) -> Result<()> {
        if let Some(ref mut rrdata) = self.rrdata {
            let name = "cube_model";
            rrdata.rruniform_buffers[image_index].update(rrdevice, ubo, name)?;
        }
        Ok(())
    }

    pub unsafe fn cleanup(&mut self, rrdevice: &RRDevice) {
        if let Some(ref mut descriptor_set) = self.descriptor_set {
            descriptor_set.destroy(&rrdevice.device);
        }
        if let Some(ref mut rrdata) = self.rrdata {
            rrdata.delete(rrdevice);
        }
        self.descriptor_set = None;
        self.rrdata = None;
    }
}
