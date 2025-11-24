#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

// include!(concat!(env!("OUT_DIR"), "\\bindings.rs"));

extern crate core;

pub mod gltf {
    pub mod gltf;
}

pub mod math {
    pub mod math;
}

pub mod fbx {
    pub mod fbx;
}

pub mod logger;
