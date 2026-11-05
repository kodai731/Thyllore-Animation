#![doc = include_str!("../README.md")]

pub const ABI_MARKER: u32 = 5;

#[cfg(feature = "python")]
pub mod python;
