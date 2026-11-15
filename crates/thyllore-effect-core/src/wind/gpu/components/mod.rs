mod shadow_volume;
mod generated {
    include!(concat!(env!("OUT_DIR"), "/wind_gpu_blocks.rs"));
}

pub use generated::*;
pub use shadow_volume::*;
