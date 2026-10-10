use crate::core::device::*;
use crate::core::swapchain::*;
use crate::data::*;
use crate::descriptor::pass_manifest::BILLBOARD;
use crate::descriptor::reflected_layout::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::descriptor::shader_bindings::billboard;
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::*;

#[derive(Clone, Debug, Default, GpuResource)]
pub struct RRBillboardDescriptorSet {
    pub layout: ReflectedSetLayout,
    #[gpu_resource(skip)]
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub rrdata: Vec<RRData>,
}

impl RRBillboardDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&BILLBOARD)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;

        Ok(Self {
            layout,
            descriptor_sets: Vec::new(),
            rrdata: Vec::new(),
        })
    }

    pub unsafe fn allocate_descriptor_sets(
        &mut self,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
    ) -> Result<()> {
        let count = self.rrdata.len() * rrswapchain.swapchain_images.len();
        self.descriptor_sets = self.layout.allocate_sets(rrdevice, count)?;
        Ok(())
    }

    pub unsafe fn update_descriptor_sets(
        &mut self,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
    ) -> Result<()> {
        let swapchain_images_len = rrswapchain.swapchain_images.len();

        for (billboard_index, rrdata) in self.rrdata.iter().enumerate() {
            for image_index in 0..swapchain_images_len {
                let descriptor_set =
                    self.descriptor_sets[billboard_index * swapchain_images_len + image_index];
                self.layout
                    .writer(descriptor_set)
                    .buffer(
                        billboard::UBO,
                        rrdata.rruniform_buffers[image_index].buffer,
                        0,
                        std::mem::size_of::<UniformBufferObject>() as u64,
                    )?
                    .apply(rrdevice);
            }
        }

        Ok(())
    }

    pub unsafe fn update_position_sampler(
        &self,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        position_image_view: vk::ImageView,
        position_sampler: vk::Sampler,
    ) -> Result<()> {
        let set_count = self.rrdata.len() * rrswapchain.swapchain_images.len();

        for descriptor_set in self.descriptor_sets.iter().take(set_count) {
            self.layout
                .writer(*descriptor_set)
                .image(
                    billboard::POSITION_SAMPLER,
                    position_image_view,
                    position_sampler,
                    vk::ImageLayout::GENERAL,
                )?
                .apply(rrdevice);
        }

        Ok(())
    }
}
