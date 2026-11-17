#![doc = include_str!("../README.md")]

mod abi_marker;
#[cfg(feature = "debug-log")]
pub mod debug_log;
#[cfg(feature = "python")]
pub mod python;

pub use abi_marker::ABI_MARKER;
