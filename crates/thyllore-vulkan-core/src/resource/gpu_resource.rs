use vulkanalia::prelude::v1_0::*;

use crate::core::device::RRDevice;

pub use thyllore_vulkan_derive::GpuResource;

pub trait GpuResource {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice);

    fn resource_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<T: GpuResource> GpuResource for Option<T> {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        if let Some(resource) = self.as_mut() {
            resource.destroy_gpu(rrdevice);
        }
        *self = None;
    }
}

impl<T: GpuResource> GpuResource for Vec<T> {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        for resource in self.iter_mut().rev() {
            resource.destroy_gpu(rrdevice);
        }
        self.clear();
    }
}

impl GpuResource for vk::Sampler {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        if *self != vk::Sampler::null() {
            rrdevice.device.destroy_sampler(*self, None);
            *self = vk::Sampler::null();
        }
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

        fn resource_name(&self) -> &'static str {
            self.name
        }
    }

    struct DefaultNamedResource;

    impl GpuResource for DefaultNamedResource {
        unsafe fn destroy_gpu(&mut self, _rrdevice: &RRDevice) {}
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
