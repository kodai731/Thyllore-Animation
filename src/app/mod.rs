pub mod data;
pub mod init;
pub mod model_loader;
pub mod render;
pub mod scene_model;
pub mod update;
pub mod cleanup;
pub mod util;
pub mod gui_data;

pub use data::AppData;
pub use gui_data::GUIData;
pub use init::*;

use crate::vulkanr::device::*;

use std::time::Instant;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Debug)]
pub struct App {
    pub entry: Entry,
    pub instance: Instance,
    pub rrdevice: RRDevice,
    pub data: AppData,
    pub frame: usize,
    pub resized: bool,
    pub start: Instant,
}
