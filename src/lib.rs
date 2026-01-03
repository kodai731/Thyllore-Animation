#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

// include!(concat!(env!("OUT_DIR"), "\\bindings.rs"));

extern crate core;

pub mod vulkanr;
pub mod loader;
pub mod debugview;
pub mod logger;
pub mod math;
pub mod scene;
