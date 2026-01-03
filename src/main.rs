#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

// マクロをインポート
#[macro_use]
extern crate rust_rendering;

mod app;
mod renderer;
mod platform;

use app::{App, GUIData};
use app::init::instance::cleanup_old_screenshots;

use anyhow::Result;

fn main() -> Result<()> {
    pretty_env_logger::init();

    cleanup_old_screenshots()?;

    // imgui
    let mut system = platform::init(file!());
    let mut gui_data = GUIData::default();

    // App
    let mut app = unsafe { App::create(&system.window)? };

    // Initialize ImGui rendering resources
    unsafe {
        App::init_imgui_rendering(
            &app.instance,
            &app.rrdevice,
            &mut app.data,
            &mut system.imgui,
        )?;
    }

    system.main_loop(&mut app, &mut gui_data);

    Ok(())
}
