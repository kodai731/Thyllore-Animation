#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

// include!(concat!(env!("OUT_DIR"), "\\bindings.rs"));

extern crate core;

pub mod vulkanr {
    pub mod core;
    pub mod resource;
    pub mod pipeline;
    pub mod descriptor;
    pub mod render;
    pub mod raytracing;
    pub mod command;
    pub mod data;
    pub mod vulkan;

    // Re-export commonly used items
    pub use core::*;
    pub use resource::*;
    pub use pipeline::*;
    pub use descriptor::*;
    pub use render::*;
    pub use raytracing::*;
    pub use command::*;
    pub use data::*;
    pub use vulkan::*;
}

pub mod loader;

pub mod math {
    pub mod math;
}

pub mod debugview;

pub mod logger;
