use anyhow::Result;

use crate::core::device::RRDevice;
use crate::descriptor::reflected_layout::{
    DescriptorSetWriter, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::vk;

#[derive(Clone, Debug, Default)]
pub struct ReflectedDescriptorSets {
    layout: ReflectedSetLayout,
    sets: Vec<vk::DescriptorSet>,
}

impl ReflectedDescriptorSets {
    pub unsafe fn create(
        rrdevice: &RRDevice,
        spec: &ReflectedLayoutSpec,
        count: usize,
    ) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, spec)?;
        let sets = layout.allocate_sets(rrdevice, count.max(1))?;
        Ok(Self { layout, sets })
    }

    pub fn get(&self, index: usize) -> Option<vk::DescriptorSet> {
        self.sets.get(index).copied()
    }

    pub fn set(&self, index: usize) -> vk::DescriptorSet {
        self.sets[index]
    }

    pub fn writer(&self, index: usize) -> DescriptorSetWriter<'_> {
        self.layout.writer(self.sets[index])
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        &self.layout
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.layout.destroy(device);
    }
}

impl GpuResource for ReflectedDescriptorSets {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
