use vulkanalia::vk;

use crate::ecs::component::WaterTorusEffect;
use crate::gpu_primitive_source;
use crate::hooks::gpu_primitive::GpuPrimitiveSource;
use thyllore_vulkan_core::raytracing::{BlasGeometry, GpuPrimitive};

impl GpuPrimitiveSource for WaterTorusEffect {
    fn gpu_primitive(&self) -> GpuPrimitive<'static> {
        let model = thyllore_effect_core::build_water_ubo(self, 0).model;
        let extent = self.major_radius + self.minor_radius;

        GpuPrimitive {
            geometry: BlasGeometry::ProceduralAabb {
                aabb: vk::AabbPositionsKHR {
                    min_x: -extent,
                    min_y: -self.minor_radius,
                    min_z: -extent,
                    max_x: extent,
                    max_y: self.minor_radius,
                    max_z: extent,
                },
            },
            model,
            base_color: [1.0, 1.0, 1.0, 1.0],
            params: [1.0, self.major_radius, self.minor_radius, 0.0],
        }
    }
}

gpu_primitive_source!(WaterTorusEffect);
