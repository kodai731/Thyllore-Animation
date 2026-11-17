use std::ffi::c_void;
use std::mem::{size_of, size_of_val};

use anyhow::Result;
use glam::{Mat4, Vec4};
use vulkanalia::prelude::v1_0::*;

use crate::command::RRCommandPool;
use crate::core::RRDevice;
use crate::data::{Vertex, VertexAttributes, VertexData, VertexGeometry};
use crate::resource::Rgba8Texture;
use crate::resource::{
    GpuResource, RRIndexBuffer, RRVertexBuffer, SkinBuffers, SkinStreamAddresses,
    SkinStreamsVersion,
};
use crate::types::Instance;
use thyllore_model_core::{
    collect_rest_geometry, pack_skin_influences, skin_geometry, write_vertex_attributes,
    write_vertex_geometry, MeshMorph, SkeletonId, SkinData,
};

/// Whether a skinning dispatch must first copy freshly staged rest geometry and influences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinStreamsUpload {
    Unchanged,
    Staged,
}

#[derive(Clone, Debug)]
pub struct MeshBuffer {
    pub geometry_buffer: RRVertexBuffer,
    pub attribute_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
    pub vertex_data: VertexData,
    pub(crate) image: vk::Image,
    pub(crate) image_memory: vk::DeviceMemory,
    pub mip_level: u32,
    pub(crate) image_view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
    pub render_to_gbuffer: bool,
    pub object_index: usize,
    skin_data: Option<SkinData>,
    skin_revision: u64,
    skin_palette: Vec<Mat4>,
    skin_buffers: Option<SkinBuffers>,
    pub skeleton_id: Option<SkeletonId>,
    pub node_index: Option<usize>,
    pub base_vertices: Vec<Vertex>,
    pub base_colors: Option<Vec<Vec4>>,
    pub morph: MeshMorph,
}

impl Default for MeshBuffer {
    fn default() -> Self {
        Self {
            geometry_buffer: RRVertexBuffer::default(),
            attribute_buffer: RRVertexBuffer::default(),
            index_buffer: RRIndexBuffer::default(),
            vertex_data: VertexData::default(),
            image: vk::Image::null(),
            image_memory: vk::DeviceMemory::null(),
            mip_level: 0,
            image_view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
            render_to_gbuffer: true,
            object_index: 0,
            skin_data: None,
            skin_revision: 0,
            skin_palette: Vec::new(),
            skin_buffers: None,
            skeleton_id: None,
            node_index: None,
            base_vertices: Vec::new(),
            base_colors: None,
            morph: MeshMorph::default(),
        }
    }
}

impl MeshBuffer {
    pub fn with_texture(texture: &Rgba8Texture) -> Self {
        let mut mesh = Self::default();
        mesh.image = texture.image;
        mesh.image_memory = texture.memory;
        mesh.mip_level = texture.mip_levels;
        mesh.image_view = texture.view;
        mesh.sampler = texture.sampler;
        mesh
    }

    #[inline]
    pub fn image_view(&self) -> crate::types::ImageView {
        crate::types::ImageView(self.image_view)
    }

    #[inline]
    pub fn sampler(&self) -> crate::types::Sampler {
        crate::types::Sampler(self.sampler)
    }

    pub unsafe fn create_vertex_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
    ) -> Result<()> {
        let geometry = self.posed_geometry();
        self.geometry_buffer = RRVertexBuffer::new(
            instance,
            rrdevice,
            command_pool,
            size_of_val(geometry.as_slice()) as u64,
            geometry.as_ptr() as *const c_void,
            geometry.len(),
        )?;

        let attributes = collect_attributes(&self.vertex_data.vertices);
        self.attribute_buffer = RRVertexBuffer::new(
            instance,
            rrdevice,
            command_pool,
            size_of_val(attributes.as_slice()) as u64,
            attributes.as_ptr() as *const c_void,
            attributes.len(),
        )?;

        Ok(())
    }

    pub fn skin_data(&self) -> Option<&SkinData> {
        self.skin_data.as_ref()
    }

    pub fn edit_skin_data(&mut self) -> Option<&mut SkinData> {
        self.skin_revision += 1;
        self.skin_data.as_mut()
    }

    pub fn set_skin_data(&mut self, skin_data: Option<SkinData>) {
        self.skin_revision += 1;
        self.skin_data = skin_data;
    }

    pub fn pose_skin(&mut self, skin_palette: Vec<Mat4>) {
        self.skin_palette = skin_palette;
    }

    pub fn is_skinned_on_device(&self) -> bool {
        self.skin_data.is_some() && !self.skin_palette.is_empty()
    }

    /// False while the device skins from an older skin than `skin_data`, so the mesh must be skinned again.
    pub fn device_skin_is_current(&self) -> bool {
        self.skin_buffers
            .as_ref()
            .is_some_and(|buffers| buffers.holds(self.skin_streams_version()))
    }

    /// The geometry the device draws: skinned by the current palette, or the vertices as stored.
    pub fn posed_geometry(&self) -> Vec<VertexGeometry> {
        let mut geometry = collect_geometry(&self.vertex_data.vertices);
        let Some(skin_data) = self
            .skin_data
            .as_ref()
            .filter(|_| self.is_skinned_on_device())
        else {
            return geometry;
        };

        let bone_count = self.skin_palette.len() - 1;
        skin_geometry(
            &collect_rest_geometry(skin_data),
            &pack_skin_influences(skin_data, bone_count),
            &self.skin_palette,
            &mut geometry,
        );
        geometry
    }

    fn skin_streams_version(&self) -> SkinStreamsVersion {
        SkinStreamsVersion {
            skin_revision: self.skin_revision,
            palette_len: self.skin_palette.len(),
        }
    }

    pub(crate) unsafe fn stage_skinning(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        frame_slot: usize,
    ) -> Result<SkinStreamsUpload> {
        let Some(skin_data) = self.skin_data.as_ref() else {
            anyhow::bail!("stage_skinning on a mesh without skin data");
        };
        let version = self.skin_streams_version();

        if let Some(outgrown) = self
            .skin_buffers
            .take_if(|buffers| buffers.palette_len() != version.palette_len)
            .as_mut()
        {
            rrdevice.wait_idle()?;
            outgrown.destroy_gpu(rrdevice);
        }
        let buffers = match &mut self.skin_buffers {
            Some(buffers) => buffers,
            vacant => vacant.insert(SkinBuffers::allocate(
                instance,
                rrdevice,
                skin_data.base_positions.len(),
                version.palette_len,
            )?),
        };

        let streams = if buffers.holds(version) {
            SkinStreamsUpload::Unchanged
        } else {
            buffers.stage_streams(instance, rrdevice, frame_slot, skin_data, version)?;
            SkinStreamsUpload::Staged
        };
        buffers.write_palette(rrdevice, frame_slot, &self.skin_palette)?;

        Ok(streams)
    }

    pub(crate) unsafe fn record_skin_stream_copies(
        &self,
        frame_slot: usize,
        device: &vulkanalia::Device,
        command_buffer: vk::CommandBuffer,
    ) {
        if let Some(buffers) = &self.skin_buffers {
            buffers.record_stream_copies(frame_slot, device, command_buffer);
        }
    }

    pub(crate) unsafe fn skin_stream_addresses(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
    ) -> Option<SkinStreamAddresses> {
        self.skin_buffers
            .as_ref()
            .map(|buffers| buffers.addresses(rrdevice, frame_slot))
    }

    pub(crate) unsafe fn stage_geometry(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        frame_slot: usize,
    ) -> Result<()> {
        let vertices = &self.vertex_data.vertices;
        self.geometry_buffer
            .stage(instance, rrdevice, frame_slot, vertices.len(), |geometry| {
                write_vertex_geometry(vertices, geometry)
            })
    }

    pub(crate) unsafe fn record_geometry_copy(
        &self,
        frame_slot: usize,
        device: &vulkanalia::Device,
        command_buffer: vk::CommandBuffer,
    ) {
        let size = (size_of::<VertexGeometry>() * self.vertex_data.vertices.len()) as u64;
        self.geometry_buffer
            .record_copy(frame_slot, device, command_buffer, size);
    }

    pub unsafe fn upload_vertices(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
    ) -> Result<()> {
        let geometry = self.posed_geometry();
        self.geometry_buffer.update(
            instance,
            rrdevice,
            command_pool,
            size_of_val(geometry.as_slice()) as u64,
            geometry.as_ptr() as *const c_void,
            geometry.len(),
        )?;

        self.upload_attributes(instance, rrdevice, command_pool)
    }

    pub unsafe fn upload_attributes(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
    ) -> Result<()> {
        let attributes = collect_attributes(&self.vertex_data.vertices);
        self.attribute_buffer.update(
            instance,
            rrdevice,
            command_pool,
            size_of_val(attributes.as_slice()) as u64,
            attributes.as_ptr() as *const c_void,
            attributes.len(),
        )
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        if self.image_view != vk::ImageView::null() {
            rrdevice.device().destroy_image_view(self.image_view, None);
            self.image_view = vk::ImageView::null();
        }
        if self.sampler != vk::Sampler::null() {
            rrdevice.device().destroy_sampler(self.sampler, None);
            self.sampler = vk::Sampler::null();
        }
        self.skin_buffers.destroy_gpu(rrdevice);
        self.skin_buffers = None;
        self.geometry_buffer.destroy(rrdevice);
        self.attribute_buffer.destroy(rrdevice);
        self.index_buffer.destroy(rrdevice);
        if self.image != vk::Image::null() {
            rrdevice.device().destroy_image(self.image, None);
            self.image = vk::Image::null();
        }
        if self.image_memory != vk::DeviceMemory::null() {
            rrdevice.device().free_memory(self.image_memory, None);
            self.image_memory = vk::DeviceMemory::null();
        }
    }
}

impl Drop for MeshBuffer {
    fn drop(&mut self) {
        if self.geometry_buffer.buffer != vk::Buffer::null() {
            log_warn!("MeshBuffer dropped without calling destroy()");
        }
    }
}

fn collect_geometry(vertices: &[Vertex]) -> Vec<VertexGeometry> {
    let mut geometry = vec![VertexGeometry::default(); vertices.len()];
    write_vertex_geometry(vertices, &mut geometry);
    geometry
}

fn collect_attributes(vertices: &[Vertex]) -> Vec<VertexAttributes> {
    let mut attributes = vec![VertexAttributes::default(); vertices.len()];
    write_vertex_attributes(vertices, &mut attributes);
    attributes
}
