#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

// include!(concat!(env!("OUT_DIR"), "\\bindings.rs"));

extern crate core;

pub mod vulkanr {
    pub mod acceleration_structure;
    pub mod buffer;
    pub mod command;
    pub mod data;
    pub mod descriptor;
    pub mod device;
    pub mod image;
    pub mod pipeline;
    pub mod render;
    pub mod swapchain;
    pub mod vulkan;
    pub mod window;
}

pub mod loader;

pub mod math {
    pub mod math;
}

pub mod debugview;

pub mod logger;
