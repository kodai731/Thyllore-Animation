use glam::{Mat4, Vec4};
use thyllore_spirv_reflect::declare_gpu_block;

declare_gpu_block! {
    #[derive(Clone, Debug, Copy)]
    pub struct FrameUBO {
        pub view: Mat4,
        pub proj: Mat4,
        pub camera_pos: Vec4,
        pub light_pos: Vec4,
        pub light_color: Vec4,
    }
}

impl Default for FrameUBO {
    fn default() -> Self {
        Self {
            view: Mat4::IDENTITY,
            proj: Mat4::IDENTITY,
            camera_pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
            light_pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
            light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

declare_gpu_block! {
    #[derive(Clone, Debug, Copy)]
    pub struct ObjectUBO {
        pub model: Mat4,
    }
}

impl Default for ObjectUBO {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY,
        }
    }
}

declare_gpu_block! {
    #[derive(Clone, Debug, Copy)]
    pub struct MaterialUBO {
        pub base_color: Vec4,
        pub metallic: f32,
        pub roughness: f32,
        pub _padding: [f32; 2],
    }
}

impl Default for MaterialUBO {
    fn default() -> Self {
        Self {
            base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            _padding: [0.0; 2],
        }
    }
}
