mod generated {
    include!(concat!(env!("OUT_DIR"), "/wind_gpu_blocks.rs"));
}

pub use generated::*;
pub mod shadow_volume;
pub use shadow_volume::*;
