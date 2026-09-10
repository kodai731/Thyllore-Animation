use crate::core::device::*;
use crate::descriptor::flame::create_scene_depth_sampler;
use crate::descriptor::pass_manifest::{WIND_RESOLVE, WIND_SHADOW_BAKE, WIND_UPSAMPLE};
use crate::descriptor::reflected_layout::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::descriptor::shader_bindings::{wind_resolve, wind_shadow_bake, wind_upsample};
use crate::resource::image::create_nearest_sampler;
use crate::resource::uniform_buffer::UniformBuffer;
use crate::resource::wind_buffer::WindBuffer;
use crate::vulkan::*;
use thyllore_effect_core::WindUBO;

#[derive(Clone, Debug, Default)]
pub struct RRWindDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: vk::DescriptorSet,
    pub scene_depth_sampler: vk::Sampler,
}

impl RRWindDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_RESOLVE).with_override(
            wind_resolve::WIND,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let sets = layout.allocate_sets(rrdevice, 1)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set: sets[0],
            scene_depth_sampler,
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        wind_buffer: &WindBuffer,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .uniform_dynamic(wind_resolve::WIND, wind_ubo)?
            .apply(rrdevice);
        self.update_shadow_volume(rrdevice, wind_buffer)?;
        self.update_scene_depth(rrdevice, scene_depth_view)
    }

    pub unsafe fn update_shadow_volume(
        &self,
        rrdevice: &RRDevice,
        wind_buffer: &WindBuffer,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .image(
                wind_resolve::SHADOW_VOLUME_SAMPLER,
                wind_buffer.shadow_volume_view,
                wind_buffer.shadow_volume_sampler,
                vk::ImageLayout::GENERAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .image(
                wind_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.layout.destroy(device);
        device.destroy_sampler(self.scene_depth_sampler, None);
    }
}

#[derive(Clone, Debug, Default)]
pub struct RRWindShadowBakeDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: vk::DescriptorSet,
}

impl RRWindShadowBakeDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_SHADOW_BAKE).with_override(
            wind_shadow_bake::WIND,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        wind_buffer: &WindBuffer,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .uniform_dynamic(wind_shadow_bake::WIND, wind_ubo)?
            .apply(rrdevice);
        self.update_shadow_volume(rrdevice, wind_buffer)
    }

    pub unsafe fn update_shadow_volume(
        &self,
        rrdevice: &RRDevice,
        wind_buffer: &WindBuffer,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .image(
                wind_shadow_bake::SHADOW_VOLUME_IMAGE,
                wind_buffer.shadow_volume_view,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.layout.destroy(device);
    }
}

#[derive(Clone, Debug, Default)]
pub struct RRWindUpsampleDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: vk::DescriptorSet,
    pub wind_color_sampler: vk::Sampler,
    pub scene_depth_sampler: vk::Sampler,
}

impl RRWindUpsampleDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_UPSAMPLE)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;
        let wind_color_sampler = create_nearest_sampler(rrdevice)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
            wind_color_sampler,
            scene_depth_sampler,
        })
    }

    pub unsafe fn update_image_views(
        &self,
        rrdevice: &RRDevice,
        wind_color_view: vk::ImageView,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .image(
                wind_upsample::WIND_COLOR_SAMPLER,
                wind_color_view,
                self.wind_color_sampler,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .image(
                wind_upsample::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.layout.destroy(device);
        device.destroy_sampler(self.wind_color_sampler, None);
        device.destroy_sampler(self.scene_depth_sampler, None);
    }
}
