use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::image::{create_image_3d, create_image_view_3d};
use crate::resource::GpuResource;
use crate::types::Instance;

#[derive(Clone, Copy, Debug)]
pub enum VolumeUsage {
    StorageSampled,
    TransferDstSampled,
    TransferSrc,
    None,
}

impl VolumeUsage {
    pub(crate) fn to_flags(self) -> vk::ImageUsageFlags {
        match self {
            VolumeUsage::StorageSampled => {
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED
            }
            VolumeUsage::TransferDstSampled => {
                vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED
            }
            VolumeUsage::TransferSrc => vk::ImageUsageFlags::TRANSFER_SRC,
            VolumeUsage::None => vk::ImageUsageFlags::empty(),
        }
    }
}

/// A 3D image with its view and a linear clamp-to-edge sampler.
#[derive(Clone, Debug, Default)]
pub struct VolumeImage {
    pub(crate) image: vk::Image,
    pub(crate) memory: vk::DeviceMemory,
    pub(crate) view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
}

unsafe fn create_linear_clamp_sampler(rrdevice: &RRDevice) -> Result<vk::Sampler> {
    let info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .max_anisotropy(1.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0);
    Ok(rrdevice.device().create_sampler(&info, None)?)
}

impl VolumeImage {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        extent: crate::types::Extent3D,
        format: crate::types::Format,
        usage: VolumeUsage,
    ) -> Result<Self> {
        let (image, memory) = create_image_3d(
            instance,
            rrdevice,
            extent.raw(),
            format.raw(),
            usage.to_flags(),
        )?;
        let view = create_image_view_3d(rrdevice, image, format.raw())?;
        let sampler = create_linear_clamp_sampler(rrdevice)?;

        Ok(Self {
            image,
            memory,
            view,
            sampler,
        })
    }

    pub fn view(&self) -> crate::types::ImageView {
        crate::types::ImageView(self.view)
    }

    pub fn sampler(&self) -> crate::types::Sampler {
        crate::types::Sampler(self.sampler)
    }

    pub(crate) fn subresource_range(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    pub(crate) unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        if self.sampler != vk::Sampler::null() {
            device.destroy_sampler(self.sampler, None);
            self.sampler = vk::Sampler::null();
        }
        if self.view != vk::ImageView::null() {
            device.destroy_image_view(self.view, None);
            self.view = vk::ImageView::null();
        }
        if self.image != vk::Image::null() {
            device.destroy_image(self.image, None);
            self.image = vk::Image::null();
        }
        if self.memory != vk::DeviceMemory::null() {
            device.free_memory(self.memory, None);
            self.memory = vk::DeviceMemory::null();
        }
    }
}

impl GpuResource for VolumeImage {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device());
    }

    fn is_destroyed(&self) -> bool {
        self.sampler.is_destroyed()
            && self.view.is_destroyed()
            && self.image.is_destroyed()
            && self.memory.is_destroyed()
    }
}
