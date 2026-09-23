mod branch_default;
mod generated {
    include!(concat!(env!("OUT_DIR"), "/flame_gpu_blocks.rs"));
}

pub use generated::*;
