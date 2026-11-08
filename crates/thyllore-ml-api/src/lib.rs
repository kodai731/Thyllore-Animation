#![doc = include_str!("../README.md")]

pub const ABI_MARKER: u32 = 6;

#[cfg(feature = "debug-log")]
pub mod debug_log;
#[cfg(feature = "python")]
pub mod python;
