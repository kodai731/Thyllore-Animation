use crate::app::App;
use crate::ecs::Resource;
use crate::hooks::gpu_resource::GpuResourceHooks;
use crate::vulkanr::command::RRCommandBuffer;
use crate::vulkanr::context::{
    CommandState, FrameSync, RenderTargets, SurfaceState, SwapchainState,
};
use crate::vulkanr::render::framebuffer::{create_color_objects, create_framebuffers};
use crate::vulkanr::render::pass::create_depth_objects;
use crate::vulkanr::resource::{destroy_all_in_reverse, GpuResource};
use crate::vulkanr::swapchain::RRSwapchain;

use anyhow::Result;
use thyllore_vulkan_core::core::destroy_surface_and_instance;
use thyllore_vulkan_core::types::Fence;
use winit::window::Window;

impl App {
    pub unsafe fn destroy(&mut self) {
        log!("Destroying application resources...");

        let _ = self.rrdevice.wait_idle();

        match GpuResourceHooks::collect() {
            Ok(hooks) => hooks.destroy_all(&self.data.ecs_world, &self.rrdevice),
            Err(error) => log_warn!("GPU resource hooks not collected: {:?}", error),
        }

        let mut resources: [&mut dyn GpuResource; 2] =
            [&mut self.gpu_timestamp_profiler, &mut self.data];
        destroy_all_in_reverse(&mut resources, &self.rrdevice);

        self.destroy_world_resource::<RenderTargets>();
        self.destroy_world_resource::<CommandState>();
        self.destroy_world_resource::<FrameSync>();
        self.destroy_world_resource::<SwapchainState>();

        self.rrdevice.destroy();
        log!("Destroyed device");

        destroy_surface_and_instance(
            &self.instance,
            self.data.ecs_world.remove_resource::<SurfaceState>(),
        );
        log!("All application resources destroyed");
    }

    unsafe fn destroy_world_resource<R: Resource + GpuResource>(&mut self) {
        if let Some(mut resource) = self.data.ecs_world.remove_resource::<R>() {
            log!("Destroying {}", resource.resource_name());
            resource.destroy_gpu(&self.rrdevice);
        }
    }

    pub unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        self.rrdevice.wait_idle()?;

        {
            let mut render_targets = self.resource_mut::<RenderTargets>();
            render_targets.render.destroy_size_dependent(&self.rrdevice);
        }

        {
            let mut swapchain_state = self.resource_mut::<SwapchainState>();
            swapchain_state.swapchain.destroy(&self.rrdevice);
        }

        let command_pool = {
            let command_state = self.resource::<CommandState>();
            self.rrdevice.free_command_buffers(
                command_state.pool.command_pool(),
                &command_state.buffers.command_buffers(),
            );
            std::rc::Rc::clone(&command_state.pool)
        };

        let surface = self
            .resource::<crate::vulkanr::context::SurfaceState>()
            .surface();
        let new_swapchain = RRSwapchain::new(window, &self.instance, &surface, &self.rrdevice)?;
        let image_count = new_swapchain.swapchain_images().len();

        {
            let mut render_targets = self.resource_mut::<RenderTargets>();
            create_depth_objects(
                &self.instance,
                &self.rrdevice,
                &new_swapchain,
                &command_pool,
                &mut render_targets.render,
            )?;
            create_color_objects(
                &self.instance,
                &self.rrdevice,
                &new_swapchain,
                &mut render_targets.render,
            )?;
            create_framebuffers(&self.rrdevice, &new_swapchain, &mut render_targets.render)?;
        }

        {
            let mut command_state = self.resource_mut::<CommandState>();
            let render_targets = self.resource::<RenderTargets>();
            RRCommandBuffer::allocate_command_buffers(
                &self.rrdevice,
                &render_targets.render,
                &mut command_state.buffers,
            )?;
        }

        {
            let mut swapchain_state = self.resource_mut::<SwapchainState>();
            swapchain_state.swapchain = new_swapchain;
            swapchain_state.images_in_flight = vec![Fence::default(); image_count];
        }

        log!("Swapchain recreated successfully");
        Ok(())
    }
}
