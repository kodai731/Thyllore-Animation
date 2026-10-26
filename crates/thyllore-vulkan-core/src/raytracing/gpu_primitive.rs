use vulkanalia::vk;

#[derive(Clone, Debug)]
pub enum BlasGeometry<'a> {
    Triangles {
        vertex_buffer: &'a vk::Buffer,
        vertex_count: u32,
        vertex_stride: u32,
        index_buffer: &'a vk::Buffer,
        index_count: u32,
    },
    ProceduralAabb {
        aabb: vk::AabbPositionsKHR,
    },
}

impl BlasGeometry<'_> {
    pub fn procedural_aabb(min: [f32; 3], max: [f32; 3]) -> Self {
        Self::ProceduralAabb {
            aabb: vk::AabbPositionsKHR {
                min_x: min[0],
                min_y: min[1],
                min_z: min[2],
                max_x: max[0],
                max_y: max[1],
                max_z: max[2],
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuPrimitive<'a> {
    pub geometry: BlasGeometry<'a>,
    pub model: cgmath::Matrix4<f32>,
    pub base_color: [f32; 4],
    pub params: [f32; 4],
    /// Device address of the effect's own instance block, read by its closest hit shader; 0 when none.
    pub effect_data_address: vk::DeviceAddress,
}
