use crate::vulkan::vk;

#[derive(Clone)]
pub struct Entry(pub(crate) vulkanalia::Entry);
impl Entry {
    #[inline]
    pub(crate) fn raw(&self) -> &vulkanalia::Entry {
        &self.0
    }
}

#[derive(Clone)]
pub struct Instance(pub(crate) vulkanalia::Instance);
impl Instance {
    #[inline]
    pub(crate) fn raw(&self) -> &vulkanalia::Instance {
        &self.0
    }
}

pub type CommandBuffer = vk::CommandBuffer;
pub type Format = vk::Format;
pub type Image = vk::Image;
pub type ImageView = vk::ImageView;
pub type Sampler = vk::Sampler;
pub type Buffer = vk::Buffer;
pub type DeviceMemory = vk::DeviceMemory;
pub type DeviceAddress = vk::DeviceAddress;
pub type DeviceSize = vk::DeviceSize;
pub type DescriptorSet = vk::DescriptorSet;
pub type RenderPass = vk::RenderPass;
pub type Framebuffer = vk::Framebuffer;
pub type Pipeline = vk::Pipeline;
pub type PipelineLayout = vk::PipelineLayout;
pub type AccelerationStructure = vk::AccelerationStructureKHR;
pub type TransformMatrix = vk::TransformMatrixKHR;
pub type Extent2D = vk::Extent2D;
pub type Extent3D = vk::Extent3D;
pub type Rect2D = vk::Rect2D;
pub type Offset2D = vk::Offset2D;
pub type Offset3D = vk::Offset3D;
pub type Viewport = vk::Viewport;
pub type SampleCountFlags = vk::SampleCountFlags;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Surface(pub(crate) vk::SurfaceKHR);
impl Surface {
    #[inline]
    pub(crate) fn raw(self) -> vk::SurfaceKHR {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DebugMessenger(pub(crate) vk::DebugUtilsMessengerEXT);
impl DebugMessenger {
    #[inline]
    pub(crate) fn raw(self) -> vk::DebugUtilsMessengerEXT {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Semaphore(pub(crate) vk::Semaphore);
impl Semaphore {
    #[inline]
    pub(crate) fn raw(self) -> vk::Semaphore {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Fence(pub(crate) vk::Fence);
impl Fence {
    #[inline]
    pub(crate) fn raw(self) -> vk::Fence {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommandPool(pub(crate) vk::CommandPool);
impl CommandPool {
    #[inline]
    pub(crate) fn raw(self) -> vk::CommandPool {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Swapchain(pub(crate) vk::SwapchainKHR);
impl Swapchain {
    #[inline]
    pub(crate) fn raw(self) -> vk::SwapchainKHR {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayout(pub(crate) vk::DescriptorSetLayout);
impl DescriptorSetLayout {
    #[inline]
    pub(crate) fn raw(self) -> vk::DescriptorSetLayout {
        self.0
    }
}
