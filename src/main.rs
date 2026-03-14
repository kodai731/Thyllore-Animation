#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

#[macro_use]
extern crate rust_rendering;

use rust_rendering::app::init::instance::cleanup_old_screenshots;
use rust_rendering::app::{App, GUIData};
use rust_rendering::platform;

use anyhow::Result;

fn main() -> Result<()> {
    pretty_env_logger::init();

    cleanup_old_screenshots()?;

    // imgui
    let window_title = format!(
        "Rust Rendering v{}",
        env!("CARGO_PKG_VERSION")
    );
    let mut system = platform::init(&window_title);
    let mut gui_data = GUIData::default();

    // App
    let mut app = unsafe { App::create(&system.window)? };

    // Initialize ImGui rendering resources
    unsafe {
        let command_pool = app.command_state().pool.clone();
        let rrrender = app.render_targets().render.clone();
        App::init_imgui_rendering(
            &app.instance,
            &app.rrdevice,
            &mut app.data,
            &mut system.imgui,
            &command_pool,
            &rrrender,
        )?;
    }

    system.main_loop(&mut app, &mut gui_data);

    Ok(())
}
