pub mod builder;
pub mod cache;
pub mod raytracing;

pub use builder::*;
pub use cache::*;
pub use raytracing::{max_ray_recursion_depth, RRRayTracingPipeline};

use vulkanalia::prelude::v1_0::*;

pub fn compute_push_constant_range(size: u32) -> vk::PushConstantRange {
    vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .offset(0)
        .size(size)
        .build()
}
