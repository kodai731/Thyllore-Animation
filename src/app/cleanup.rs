use crate::app::App;
use rust_rendering::vulkanr::device::*;

use vulkanalia::prelude::v1_0::*;

impl App {
    unsafe fn destroy(&mut self) {
        log!("Destroying application resources...");

        if let Some(sampler) = self.data.raytracing.gbuffer_sampler {
            self.rrdevice.device.destroy_sampler(sampler, None);
            log!("Destroyed G-Buffer sampler");
        }

        if let Some(mut gbuffer_descriptor) = self.data.raytracing.gbuffer_descriptor_set.take() {
            gbuffer_descriptor.destroy(&self.rrdevice.device);
            log!("Destroyed G-Buffer descriptor set");
        }

        if let Some(gbuffer_pipeline) = self.data.raytracing.gbuffer_pipeline.take() {
            gbuffer_pipeline.destroy(&self.rrdevice.device);
            log!("Destroyed G-Buffer pipeline");
        }

        if let Some(mut composite_descriptor) = self.data.raytracing.composite_descriptor.take() {
            composite_descriptor.destroy(&self.rrdevice.device);
            log!("Destroyed composite descriptor set");
        }

        if let Some(composite_pipeline) = self.data.raytracing.composite_pipeline.take() {
            composite_pipeline.destroy(&self.rrdevice.device);
            log!("Destroyed composite pipeline");
        }

        if let (Some(buffer), Some(memory)) = (
            self.data.raytracing.scene_uniform_buffer,
            self.data.raytracing.scene_uniform_buffer_memory,
        ) {
            self.rrdevice.device.destroy_buffer(buffer, None);
            self.rrdevice.device.free_memory(memory, None);
            log!("Destroyed scene uniform buffer");
        }

        if let Some(mut ray_query_descriptor) = self.data.raytracing.ray_query_descriptor.take() {
            ray_query_descriptor.destroy(&self.rrdevice.device);
            log!("Destroyed ray query descriptor set");
        }

        if let Some(ray_query_pipeline) = self.data.raytracing.ray_query_pipeline.take() {
            ray_query_pipeline.destroy(&self.rrdevice.device);
            log!("Destroyed ray query pipeline");
        }

        if let Some(mut acceleration_structure) = self.data.raytracing.acceleration_structure.take() {
            acceleration_structure.destroy(&self.rrdevice.device);
            log!("Destroyed acceleration structure");
        }

        if let Some(mut gbuffer) = self.data.raytracing.gbuffer.take() {
            gbuffer.destroy(&*self.rrdevice.device);
            log!("Destroyed G-Buffer");
        }

        log!("All application resources destroyed");
    }

    //unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
    // self.rrdevice.device.device_wait_idle()?;
    // self.destroy_swapchain();
    // Self::create_swapchain(
    //     window,
    //     &self.instance,
    //     &self.rrdevice.device,
    //     &mut self.data,
    // )?;
    // Self::create_swapchain_image_view(&self.rrdevice.device, &mut self.data)?;
    // Self::create_render_pass(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_pipeline(&self.rrdevice.device, &mut self.data)?;
    // Self::create_color_objects(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_depth_objects(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_framebuffers(&self.rrdevice.device, &mut self.data)?;
    // Self::create_uniform_buffers(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_uniform_buffers_grid(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_descriptor_pool(&self.rrdevice.device, &mut self.data)?;
    // Self::create_descriptor_sets(&self.rrdevice.device, &mut self.data)?;
    // Self::create_descriptor_sets_grid(&self.rrdevice.device, &mut self.data)?;
    // Self::create_command_buffers(&self.rrdevice.device, &mut self.data)?;
    // self.data
    //     .images_in_flight
    //     .resize(self.data.swapchain_images.len(), vk::Fence::null());
    //
    //Ok(())
    // }

    unsafe fn destroy_swapchain(&mut self) {
        // // depth objects
        // self.rrdevice
        //     .device
        //     .destroy_image(self.data.depth_image, None);
        // self.rrdevice
        //     .device
        //     .free_memory(self.data.depth_image_memory, None);
        // self.rrdevice
        //     .device
        //     .destroy_image_view(self.data.depth_image_view, None);
        // // color objects
        // self.rrdevice
        //     .device
        //     .destroy_image(self.data.color_image, None);
        // self.rrdevice
        //     .device
        //     .free_memory(self.data.color_image_memory, None);
        // self.rrdevice
        //     .device
        //     .destroy_image_view(self.data.color_image_view, None);
        // // descriptor pool
        // self.rrdevice
        //     .device
        //     .destroy_descriptor_pool(self.data.descriptor_pool, None);
        // // uniform buffers
        // self.data
        //     .uniform_buffers
        //     .iter()
        //     .for_each(|b| self.rrdevice.device.destroy_buffer(*b, None));
        // self.data
        //     .uniform_buffer_memories
        //     .iter()
        //     .for_each(|m| self.rrdevice.device.free_memory(*m, None));
        // // framebuffers
        // self.data
        //     .framebuffers
        //     .iter()
        //     .for_each(|f| self.rrdevice.device.destroy_framebuffer(*f, None));
        // // command buffers
        // self.rrdevice
        //     .device
        //     .free_command_buffers(self.data.command_pool, &self.data.command_buffers);
        // // The pipeline layout will be referenced throughout the program's lifetime
        // self.rrdevice
        //     .device
        //     .destroy_pipeline_layout(self.data.pipeline_layout, None);
        // // render pass
        // self.rrdevice
        //     .device
        //     .destroy_render_pass(self.data.render_pass, None);
        // // graphics pipeline
        // self.rrdevice
        //     .device
        //     .destroy_pipeline(self.data.pipeline, None);
        // // swapchain imageviews
        // self.data
        //     .swapchain_image_views
        //     .iter()
        //     .for_each(|v| self.rrdevice.device.destroy_image_view(*v, None));
        // // swapchain
        // self.rrdevice
        //     .device
        //     .destroy_swapchain_khr(self.data.swapchain, None);
    }
}
