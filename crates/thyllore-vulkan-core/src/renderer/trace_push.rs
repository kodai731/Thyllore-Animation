// Generated from SPIR-V by `cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks`; do not edit.
use cgmath::Matrix4;
use thyllore_spirv_reflect::declare_gpu_block;

declare_gpu_block! {
    #[derive(Clone, Copy, Debug)]
    pub struct TracePush {
        pub inv_view_proj: Matrix4<f32>,
        pub camera_pos: [f32; 4],
        pub light_pos: [f32; 4],
        pub light_color: [f32; 4],
    }
}
