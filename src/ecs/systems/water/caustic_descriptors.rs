use crate::vulkanr::core::device::*;
use crate::vulkanr::data::SceneUniformData;
use crate::vulkanr::descriptor::pass_manifest::{WATER_CAUSTIC_APPLY, WATER_CAUSTIC_SPLAT};
use crate::vulkanr::descriptor::shader_bindings::{water_caustic_apply, water_caustic_splat};
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::vulkanr::resource::gpu_resource::GpuResource;
use crate::vulkanr::vulkan::*;

#[derive(Clone, Debug, Default)]
pub struct RRWaterCausticDescriptorSet {
    splat: ReflectedDescriptorSets,
    apply: ReflectedDescriptorSets,
}

impl RRWaterCausticDescriptorSet {
    pub fn splat_layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_CAUSTIC_SPLAT)
    }

    pub fn apply_layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_CAUSTIC_APPLY)
    }

    pub fn splat_layout(&self) -> &ReflectedSetLayout {
        self.splat.layout()
    }

    pub fn splat_descriptor_set(&self) -> vk::DescriptorSet {
        self.splat.set(0)
    }

    pub fn apply_layout(&self) -> &ReflectedSetLayout {
        self.apply.layout()
    }

    pub fn apply_descriptor_set(&self) -> vk::DescriptorSet {
        self.apply.set(0)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let splat = ReflectedDescriptorSets::create(rrdevice, &Self::splat_layout_spec(), 1)?;
        let apply = ReflectedDescriptorSets::create(rrdevice, &Self::apply_layout_spec(), 1)?;

        Ok(Self { splat, apply })
    }

    pub unsafe fn allocate_and_update(
        &mut self,
        rrdevice: &RRDevice,
        caustic_accum_view: vk::ImageView,
        position_image_view: vk::ImageView,
        tlas: Option<vk::AccelerationStructureKHR>,
        scene_uniform_buffer: vk::Buffer,
        water_ubo: vk::Buffer,
        hdr_color_image_view: vk::ImageView,
    ) -> Result<()> {
        let mut splat_writer = self.splat.writer(0).image(
            water_caustic_splat::CAUSTIC_ACCUM_IMAGE,
            caustic_accum_view,
            vk::Sampler::null(),
            vk::ImageLayout::GENERAL,
        )?;
        if let Some(tlas) = tlas {
            splat_writer =
                splat_writer.acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?;
        }
        splat_writer
            .image(
                water_caustic_splat::POSITION_IMAGE,
                position_image_view,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
            )?
            .buffer(
                water_caustic_splat::SCENE_DATA,
                scene_uniform_buffer,
                0,
                std::mem::size_of::<SceneUniformData>() as u64,
            )?
            .buffer(
                water_caustic_splat::WATER_BLOCK,
                water_ubo,
                0,
                vk::WHOLE_SIZE as u64,
            )?
            .apply(rrdevice);

        self.apply
            .writer(0)
            .image(
                water_caustic_apply::CAUSTIC_ACCUM_IMAGE,
                caustic_accum_view,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
            )?
            .image(
                water_caustic_apply::HDR_COLOR_IMAGE,
                hdr_color_image_view,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
            )?
            .buffer(
                water_caustic_apply::SCENE_DATA,
                scene_uniform_buffer,
                0,
                std::mem::size_of::<SceneUniformData>() as u64,
            )?
            .buffer(
                water_caustic_apply::WATER_BLOCK,
                water_ubo,
                0,
                vk::WHOLE_SIZE as u64,
            )?
            .apply(rrdevice);

        Ok(())
    }

    pub unsafe fn update_tlas(
        &mut self,
        rrdevice: &RRDevice,
        tlas: vk::AccelerationStructureKHR,
    ) -> Result<()> {
        self.splat
            .writer(0)
            .acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.splat.destroy(device);
        self.apply.destroy(device);
    }
}

impl GpuResource for RRWaterCausticDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
