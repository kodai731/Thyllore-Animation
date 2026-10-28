use crate::core::device::*;
use crate::descriptor::pass_manifest::{AUTO_EXPOSURE_AVERAGE, AUTO_EXPOSURE_HISTOGRAM};
use crate::descriptor::reflected_layout::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::descriptor::shader_bindings::{auto_exposure_average, auto_exposure_histogram};
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::*;

#[derive(Clone, Debug, Default)]
pub struct RRAutoExposureHistogramDescriptorSet {
    pub layout: ReflectedSetLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl RRAutoExposureHistogramDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&AUTO_EXPOSURE_HISTOGRAM)
    }

    pub unsafe fn new(rrdevice: &RRDevice, set_count: usize) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_sets = layout.allocate_sets(rrdevice, set_count.max(1))?;

        Ok(Self {
            layout,
            descriptor_sets,
        })
    }

    pub fn descriptor_set(&self, frame_slot: usize) -> Result<crate::types::DescriptorSet> {
        self.descriptor_sets
            .get(frame_slot)
            .copied()
            .map(crate::types::DescriptorSet)
            .ok_or_else(|| {
                anyhow!(
                    "auto exposure histogram descriptor slot {frame_slot} exceeds {} sets",
                    self.descriptor_sets.len()
                )
            })
    }

    pub unsafe fn update_bindings(
        &self,
        rrdevice: &RRDevice,
        hdr_image_view: crate::types::ImageView,
        hdr_sampler: crate::types::Sampler,
        histogram_buffer: crate::types::Buffer,
        histogram_buffer_size: u64,
    ) -> Result<()> {
        for descriptor_set in &self.descriptor_sets {
            self.layout
                .writer(*descriptor_set)
                .image(
                    auto_exposure_histogram::HDR_IMAGE,
                    hdr_image_view.raw(),
                    hdr_sampler.raw(),
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )?
                .buffer(
                    auto_exposure_histogram::HISTOGRAM,
                    histogram_buffer.raw(),
                    0,
                    histogram_buffer_size,
                )?
                .apply(rrdevice);
        }
        Ok(())
    }

    pub unsafe fn update_hdr_image_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        hdr_image_view: crate::types::ImageView,
        hdr_sampler: crate::types::Sampler,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set(frame_slot)?.raw())
            .image(
                auto_exposure_histogram::HDR_IMAGE,
                hdr_image_view.raw(),
                hdr_sampler.raw(),
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
    }
}

#[derive(Clone, Debug, Default)]
pub struct RRAutoExposureAverageDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub(crate) descriptor_set: vk::DescriptorSet,
}

impl RRAutoExposureAverageDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&AUTO_EXPOSURE_AVERAGE)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
        })
    }

    pub unsafe fn update_bindings(
        &self,
        rrdevice: &RRDevice,
        histogram_buffer: crate::types::Buffer,
        histogram_buffer_size: u64,
        luminance_buffer: crate::types::Buffer,
        luminance_buffer_size: u64,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .buffer(
                auto_exposure_average::HISTOGRAM,
                histogram_buffer.raw(),
                0,
                histogram_buffer_size,
            )?
            .buffer(
                auto_exposure_average::RESULT,
                luminance_buffer.raw(),
                0,
                luminance_buffer_size,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
    }
}

impl GpuResource for RRAutoExposureHistogramDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed()
    }
}

impl GpuResource for RRAutoExposureAverageDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed()
    }
}
