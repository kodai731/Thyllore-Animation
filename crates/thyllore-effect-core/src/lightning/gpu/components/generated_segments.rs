// Generated from SPIR-V by `cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks`; do not edit.
use cgmath::Matrix4;
use thyllore_spirv_reflect::declare_gpu_block;

declare_gpu_block! {
    #[derive(Clone, Copy, Debug)]
    pub struct LightningSegmentsUBO {
        pub seg_a_r0: [[f32; 4]; 256],
        pub seg_b_r1: [[f32; 4]; 256],
        pub seg_misc: [[f32; 4]; 256],
    }
}
