use vulkanalia::prelude::v1_0::*;

use crate::core::device::RRDevice;

pub use thyllore_vulkan_derive::GpuResource;

/// Owns Vulkan objects; `#[derive(GpuResource)]` on a `#[must_use]` struct destroys its fields in reverse order and warns from `Drop` while any is live.
pub trait GpuResource {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice);

    /// False while this value still owns a live Vulkan object.
    fn is_destroyed(&self) -> bool;

    fn resource_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub fn warn_dropped_live(resource_name: &'static str) {
    log_warn!("{resource_name} dropped without destroy_gpu()");
}

macro_rules! raw_handle_resource {
    ($handle:ty, $destroy:ident) => {
        impl GpuResource for $handle {
            unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
                if *self != <$handle>::null() {
                    rrdevice.device.$destroy(*self, None);
                    *self = <$handle>::null();
                }
            }

            fn is_destroyed(&self) -> bool {
                *self == <$handle>::null()
            }
        }
    };
}

raw_handle_resource!(vk::Image, destroy_image);
raw_handle_resource!(vk::ImageView, destroy_image_view);
raw_handle_resource!(vk::DeviceMemory, free_memory);
raw_handle_resource!(vk::Buffer, destroy_buffer);
raw_handle_resource!(vk::Sampler, destroy_sampler);
raw_handle_resource!(vk::DescriptorSetLayout, destroy_descriptor_set_layout);
raw_handle_resource!(vk::Pipeline, destroy_pipeline);
raw_handle_resource!(vk::PipelineLayout, destroy_pipeline_layout);
raw_handle_resource!(vk::RenderPass, destroy_render_pass);
raw_handle_resource!(vk::Framebuffer, destroy_framebuffer);
raw_handle_resource!(vk::ShaderModule, destroy_shader_module);
raw_handle_resource!(vk::QueryPool, destroy_query_pool);

/// A descriptor set that returns to the device allocator of its layout when destroyed.
#[must_use]
#[derive(Clone, Copy, Debug, Default)]
pub struct OwnedDescriptorSet {
    set: vk::DescriptorSet,
    layout: vk::DescriptorSetLayout,
}

impl OwnedDescriptorSet {
    pub fn new(set: vk::DescriptorSet, layout: vk::DescriptorSetLayout) -> Self {
        Self { set, layout }
    }

    pub fn handle(&self) -> vk::DescriptorSet {
        self.set
    }
}

impl GpuResource for OwnedDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        if self.set != vk::DescriptorSet::null() {
            rrdevice.release_descriptor_sets(self.layout, [self.set]);
            self.set = vk::DescriptorSet::null();
        }
    }

    fn is_destroyed(&self) -> bool {
        self.set == vk::DescriptorSet::null()
    }
}

impl<T: GpuResource> GpuResource for Option<T> {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        if let Some(resource) = self.as_mut() {
            resource.destroy_gpu(rrdevice);
        }
        *self = None;
    }

    fn is_destroyed(&self) -> bool {
        self.as_ref().map_or(true, GpuResource::is_destroyed)
    }
}

impl<T: GpuResource> GpuResource for Vec<T> {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        for resource in self.iter_mut().rev() {
            resource.destroy_gpu(rrdevice);
        }
        self.clear();
    }

    fn is_destroyed(&self) -> bool {
        self.iter().all(GpuResource::is_destroyed)
    }
}

pub unsafe fn destroy_all_in_reverse(resources: &mut [&mut dyn GpuResource], rrdevice: &RRDevice) {
    destroy_all_in_reverse_with(resources, |resource| resource.destroy_gpu(rrdevice));
}

fn destroy_all_in_reverse_with(
    resources: &mut [&mut dyn GpuResource],
    mut destroy: impl FnMut(&mut dyn GpuResource),
) {
    for resource in resources.iter_mut().rev() {
        log!("Destroying {}", resource.resource_name());
        destroy(&mut **resource);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    struct MockResource {
        name: &'static str,
    }

    impl GpuResource for MockResource {
        unsafe fn destroy_gpu(&mut self, _rrdevice: &RRDevice) {}

        fn is_destroyed(&self) -> bool {
            true
        }

        fn resource_name(&self) -> &'static str {
            self.name
        }
    }

    struct DefaultNamedResource;

    impl GpuResource for DefaultNamedResource {
        unsafe fn destroy_gpu(&mut self, _rrdevice: &RRDevice) {}

        fn is_destroyed(&self) -> bool {
            true
        }
    }

    #[test]
    fn raw_handles_report_destroyed_only_when_null() {
        use vulkanalia::vk::Handle;
        assert!(vk::Image::null().is_destroyed());
        assert!(!vk::Image::from_raw(1).is_destroyed());
        assert!(Some(vk::Buffer::null()).is_destroyed());
        assert!(!vec![vk::Buffer::null(), vk::Buffer::from_raw(7)].is_destroyed());
        assert!(OwnedDescriptorSet::default().is_destroyed());
    }

    #[test]
    fn destroys_resources_in_reverse_order() {
        let destroyed_order: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let mut first = MockResource { name: "first" };
        let mut second = MockResource { name: "second" };
        let mut resources: [&mut dyn GpuResource; 2] = [&mut first, &mut second];

        destroy_all_in_reverse_with(&mut resources, |resource| {
            destroyed_order.borrow_mut().push(resource.resource_name())
        });

        assert_eq!(destroyed_order.borrow().as_slice(), ["second", "first"]);
    }

    #[test]
    fn resource_name_defaults_to_the_type_name() {
        assert!(DefaultNamedResource
            .resource_name()
            .ends_with("DefaultNamedResource"));
    }
}
