#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

#[macro_use]
extern crate thyllore_animation;

use thyllore_animation::app::init::instance::cleanup_old_screenshots;
use thyllore_animation::app::{App, GUIData};
use thyllore_animation::platform;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();

    cleanup_old_screenshots()?;

    // imgui
    let window_title = format!("Thyllore Animation v{}", env!("CARGO_PKG_VERSION"));
    let mut system = platform::init(&window_title);
    let mut gui_data = GUIData::default();

    // App
    let mut app = unsafe { App::create(&system.window)? };

    // Initialize ImGui rendering resources
    unsafe {
        use thyllore_animation::vulkanr::context::{CommandState, RenderTargets};
        let command_pool = app.resource::<CommandState>().pool.clone();
        let rrrender = app.resource::<RenderTargets>().render.clone();
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
