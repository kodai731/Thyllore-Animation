use vulkanalia::vk;

use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::WaterTraceBlocks;
use crate::ecs::world::World;
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
            effect_data_address: 0,
        }
    }

    fn effect_data_address(world: &World, ordinal: usize) -> vk::DeviceAddress {
        world
            .get_resource::<WaterTraceBlocks>()
            .map(|blocks| blocks.slot_address(ordinal))
            .unwrap_or(0)
    }
}

gpu_primitive_source!(WaterTorusEffect);
