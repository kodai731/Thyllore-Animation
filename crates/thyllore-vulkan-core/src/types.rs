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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Buffer(pub(crate) vk::Buffer);
impl Buffer {
    #[inline]
    pub(crate) fn raw(self) -> vk::Buffer {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeviceMemory(pub(crate) vk::DeviceMemory);
impl DeviceMemory {
    #[inline]
    pub(crate) fn raw(self) -> vk::DeviceMemory {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DescriptorSet(pub(crate) vk::DescriptorSet);
impl DescriptorSet {
    #[inline]
    pub(crate) fn raw(self) -> vk::DescriptorSet {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderPass(pub(crate) vk::RenderPass);
impl RenderPass {
    #[inline]
    pub(crate) fn raw(self) -> vk::RenderPass {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Framebuffer(pub(crate) vk::Framebuffer);
impl Framebuffer {
    #[inline]
    pub(crate) fn raw(self) -> vk::Framebuffer {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pipeline(pub(crate) vk::Pipeline);
impl Pipeline {
    #[inline]
    pub(crate) fn raw(self) -> vk::Pipeline {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PipelineLayout(pub(crate) vk::PipelineLayout);
impl PipelineLayout {
    #[inline]
    pub(crate) fn raw(self) -> vk::PipelineLayout {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AccelerationStructure(pub(crate) vk::AccelerationStructureKHR);
impl AccelerationStructure {
    #[inline]
    pub(crate) fn raw(self) -> vk::AccelerationStructureKHR {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TransformMatrix(pub(crate) vk::TransformMatrixKHR);
impl TransformMatrix {
    #[inline]
    pub(crate) fn raw(self) -> vk::TransformMatrixKHR {
        self.0
    }

    #[inline]
    pub fn matrix(&self) -> [[f32; 4]; 3] {
        self.0.matrix
    }
}

pub type DeviceAddress = vk::DeviceAddress;
pub type DeviceSize = vk::DeviceSize;

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
