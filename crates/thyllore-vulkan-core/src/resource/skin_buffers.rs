use std::mem::size_of;

use anyhow::Result;
use glam::Mat4;
use thyllore_model_core::{
    collect_rest_geometry, pack_skin_influences, SkinData, SkinInfluence, VertexGeometry,
};
use thyllore_render_core::FRAMES_IN_FLIGHT;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::{GpuResource, HostBuffer, HostBufferUsage, RRVertexBuffer};
use crate::types::Instance;

/// Which revision of a mesh's skin, packed for which palette length, the device streams hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkinStreamsVersion {
    pub(crate) skin_revision: u64,
    pub(crate) palette_len: usize,
}

/// The device addresses one skinning dispatch reads and writes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SkinStreamAddresses {
    pub(crate) rest: vk::DeviceAddress,
    pub(crate) influences: vk::DeviceAddress,
    pub(crate) palette: vk::DeviceAddress,
}

/// The inputs of palette skinning on the device: rest geometry, influences and one palette per frame slot.
#[must_use]
#[derive(Clone, Debug, Default)]
pub struct SkinBuffers {
    rest: RRVertexBuffer,
    influences: RRVertexBuffer,
    palettes: Vec<HostBuffer>,
    staged_version: Option<SkinStreamsVersion>,
}

impl SkinBuffers {
    pub(crate) unsafe fn allocate(
        instance: &Instance,
        rrdevice: &RRDevice,
        vertex_count: usize,
        palette_len: usize,
    ) -> Result<Self> {
        let rest = RRVertexBuffer::allocate(
            instance,
            rrdevice,
            (size_of::<VertexGeometry>() * vertex_count) as u64,
            vertex_count,
        )?;
        let influences = RRVertexBuffer::allocate(
            instance,
            rrdevice,
            (size_of::<SkinInfluence>() * vertex_count) as u64,
            vertex_count,
        )?;

        let mut palettes = Vec::with_capacity(FRAMES_IN_FLIGHT);
        for _ in 0..FRAMES_IN_FLIGHT {
            palettes.push(HostBuffer::create(
                instance,
                rrdevice,
                (size_of::<Mat4>() * palette_len) as u64,
                HostBufferUsage::ShaderAddressed,
            )?);
        }

        Ok(Self {
            rest,
            influences,
            palettes,
            staged_version: None,
        })
    }

    pub(crate) fn holds(&self, version: SkinStreamsVersion) -> bool {
        self.staged_version == Some(version)
    }

    pub(crate) fn palette_len(&self) -> usize {
        self.palettes
            .first()
            .map_or(0, |palette| palette.size() as usize / size_of::<Mat4>())
    }

    pub(crate) unsafe fn stage_streams(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        frame_slot: usize,
        skin_data: &SkinData,
        version: SkinStreamsVersion,
    ) -> Result<()> {
        let rest = collect_rest_geometry(skin_data);
        self.rest
            .stage(instance, rrdevice, frame_slot, rest.len(), |staged| {
                staged.copy_from_slice(&rest)
            })?;

        let bone_count = version.palette_len - 1;
        let influences = pack_skin_influences(skin_data, bone_count);
        self.influences
            .stage(instance, rrdevice, frame_slot, influences.len(), |staged| {
                staged.copy_from_slice(&influences)
            })?;

        self.staged_version = Some(version);
        Ok(())
    }

    pub(crate) unsafe fn record_stream_copies(
        &self,
        frame_slot: usize,
        device: &vulkanalia::Device,
        command_buffer: vk::CommandBuffer,
    ) {
        let vertex_count = self.rest.vertices as usize;
        self.rest.record_copy(
            frame_slot,
            device,
            command_buffer,
            (size_of::<VertexGeometry>() * vertex_count) as u64,
        );
        self.influences.record_copy(
            frame_slot,
            device,
            command_buffer,
            (size_of::<SkinInfluence>() * vertex_count) as u64,
        );
    }

    pub(crate) unsafe fn write_palette(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        palette: &[Mat4],
    ) -> Result<()> {
        let bytes = std::slice::from_raw_parts(
            palette.as_ptr().cast::<u8>(),
            std::mem::size_of_val(palette),
        );
        self.palettes[frame_slot].write_with(rrdevice, bytes.len() as u64, |mapped| {
            mapped.copy_from_slice(bytes)
        })
    }

    pub(crate) unsafe fn addresses(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
    ) -> SkinStreamAddresses {
        SkinStreamAddresses {
            rest: rrdevice.buffer_address(self.rest.buffer),
            influences: rrdevice.buffer_address(self.influences.buffer),
            palette: rrdevice.buffer_address(self.palettes[frame_slot].buffer),
        }
    }
}

impl GpuResource for SkinBuffers {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.rest.destroy(rrdevice);
        self.influences.destroy(rrdevice);
        self.palettes.destroy_gpu(rrdevice);
        self.staged_version = None;
    }

    fn is_destroyed(&self) -> bool {
        self.rest.buffer.is_destroyed()
            && self.influences.buffer.is_destroyed()
            && self.palettes.is_destroyed()
    }
}
