mod generated {
    include!(concat!(env!("OUT_DIR"), "/lightning_gpu_blocks.rs"));
}
mod generated_segments {
    include!(concat!(
        env!("OUT_DIR"),
        "/lightning_segments_gpu_blocks.rs"
    ));
}

pub use generated::*;
pub use generated_segments::*;
