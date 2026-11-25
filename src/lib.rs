#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

extern crate core;

#[macro_use]
extern crate thyllore_log_core;

pub mod app;
pub mod asset;
#[cfg(debug_assertions)]
pub mod debugview;
pub mod ecs;
pub mod effect;
pub mod hooks;
#[cfg(feature = "ml")]
pub mod ml;
pub mod platform;
pub mod scene;
pub mod scratch;
pub mod vulkanr;
