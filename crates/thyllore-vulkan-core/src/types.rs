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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommandBuffer(pub(crate) vk::CommandBuffer);
impl CommandBuffer {
    #[inline]
    pub(crate) fn raw(self) -> vk::CommandBuffer {
        self.0
    }
}

pub type Format = vk::Format;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Image(pub(crate) vk::Image);
impl Image {
    #[inline]
    pub(crate) fn raw(self) -> vk::Image {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageView(pub(crate) vk::ImageView);
impl ImageView {
    #[inline]
    pub(crate) fn raw(self) -> vk::ImageView {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sampler(pub(crate) vk::Sampler);
impl Sampler {
    #[inline]
    pub(crate) fn raw(self) -> vk::Sampler {
        self.0
    }
}
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
    pub fn matrix(&self) -> [[f32; 4]; 3] {
        self.0.matrix
    }
}

pub type DeviceAddress = u64;
pub type DeviceSize = u64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}
impl Extent2D {
    #[inline]
    pub(crate) fn raw(self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }
    #[inline]
    pub(crate) fn from_raw(raw: vk::Extent2D) -> Self {
        Self {
            width: raw.width,
            height: raw.height,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}
impl Extent3D {
    #[inline]
    pub(crate) fn raw(self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Offset2D {
    pub x: i32,
    pub y: i32,
}
impl Offset2D {
    #[inline]
    pub(crate) fn raw(self) -> vk::Offset2D {
        vk::Offset2D {
            x: self.x,
            y: self.y,
        }
    }
    #[inline]
    pub(crate) fn from_raw(raw: vk::Offset2D) -> Self {
        Self { x: raw.x, y: raw.y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Offset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
impl Offset3D {
    #[inline]
    pub(crate) fn raw(self) -> vk::Offset3D {
        vk::Offset3D {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect2D {
    pub offset: Offset2D,
    pub extent: Extent2D,
}
impl Rect2D {
    #[inline]
    pub(crate) fn raw(self) -> vk::Rect2D {
        vk::Rect2D {
            offset: self.offset.raw(),
            extent: self.extent.raw(),
        }
    }
    #[inline]
    pub(crate) fn from_raw(raw: vk::Rect2D) -> Self {
        Self {
            offset: Offset2D::from_raw(raw.offset),
            extent: Extent2D::from_raw(raw.extent),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

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
