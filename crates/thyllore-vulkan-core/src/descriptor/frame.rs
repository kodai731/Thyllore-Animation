use crate::core::device::RRDevice;
use crate::descriptor::pass_manifest::SetRole;
use crate::descriptor::reflected_layout::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::descriptor::shader_bindings::model;
use crate::resource::uniform_buffer::{Placement, UniformBuffer};
use crate::types::Instance;
use thyllore_render_core::FrameUBO;

#[derive(Clone, Debug, Default)]
pub struct FrameDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub(crate) sets: Vec<crate::types::DescriptorSet>,
    pub buffers: Vec<UniformBuffer<FrameUBO>>,
}

impl FrameDescriptorSet {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
    ) -> anyhow::Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let sets = layout
            .allocate_sets(rrdevice, swapchain_image_count)?
            .into_iter()
            .map(crate::types::DescriptorSet)
            .collect();

        let mut buffers = Vec::with_capacity(swapchain_image_count);
        for _ in 0..swapchain_image_count {
            buffers.push(UniformBuffer::new(
                instance,
                rrdevice,
                1,
                Placement::HostMapped,
            )?);
        }

        let frame_set = Self {
            layout,
            sets,
            buffers,
        };
        frame_set.write_descriptor_sets(rrdevice)?;

        Ok(frame_set)
    }

    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::shared(SetRole::Frame)
    }

    unsafe fn write_descriptor_sets(&self, rrdevice: &RRDevice) -> anyhow::Result<()> {
        for (set, buffer) in self.sets.iter().zip(&self.buffers) {
            self.layout
                .writer(set.raw())
                .uniform(model::FRAME, buffer, 0)?
                .apply(rrdevice);
        }
        Ok(())
    }

    pub unsafe fn update(
        &self,
        rrdevice: &RRDevice,
        image_index: usize,
        ubo: &FrameUBO,
    ) -> anyhow::Result<()> {
        self.buffers[image_index].write_slot(rrdevice, 0, ubo)
    }

    #[inline]
    pub fn sets(&self) -> &[crate::types::DescriptorSet] {
        &self.sets
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device();
        for buffer in &mut self.buffers {
            buffer.destroy(device);
        }
        self.layout.destroy(rrdevice);
    }
}
