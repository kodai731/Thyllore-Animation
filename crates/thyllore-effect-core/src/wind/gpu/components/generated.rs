// Generated from SPIR-V by `cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks`; do not edit.
use cgmath::Matrix4;
use thyllore_spirv_reflect::declare_gpu_block;

declare_gpu_block! {
    #[derive(Clone, Copy, Debug)]
    pub struct WindUBO {
        pub model: Matrix4<f32>,
        pub inverse_model: Matrix4<f32>,
        pub shape: [f32; 4],
        pub wall: [f32; 4],
        pub optics: [f32; 4],
        pub albedo: [f32; 4],
        pub lighting: [f32; 4],
        pub streak: [f32; 4],
        pub streak2: [f32; 4],
        pub eddy: [f32; 4],
        pub eddy2: [f32; 4],
        pub puff_params: [f32; 4],
        pub puffs: [[f32; 4]; 96],
        pub inv_view_proj: Matrix4<f32>,
    }
}
