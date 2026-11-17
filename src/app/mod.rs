pub mod bootstrap;
pub mod capture_context;
pub mod cleanup;
pub(crate) mod command;
mod command_recording;
pub mod config;
mod data;
pub mod effect_hooks;
pub(crate) mod features;
pub mod frame;
mod imgui_textures;
mod init;
mod pass_targets;
pub mod post_process;
pub mod render;
mod render_context;
pub mod scene_rebuild;
mod state;
pub mod update;
pub mod viewport;

pub use render_context::{build_frame_render_context, build_pass_context, RenderContext};

pub use data::AppData;
pub use features::screenshot::cleanup_old_screenshots;
pub use init::*;
pub use state::App;
