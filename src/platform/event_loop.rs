use std::time::Instant;

use winit::event::Event;

use super::events::dispatch_window_event;
use super::key_bindings::default_bindings;
use crate::app::App;
use crate::hooks::external_command::ExternalCommandSender;
use crate::platform::System;

impl System {
    pub fn external_command_sender(&self) -> ExternalCommandSender {
        ExternalCommandSender::new(self.event_loop.create_proxy())
    }

    pub fn main_loop(self, app: &mut App) {
        let System {
            event_loop,
            window,
            mut imgui,
            mut platform,
            ui_fonts: _ui_fonts,
        } = self;
        let mut last_frame = Instant::now();
        let bindings = default_bindings();

        event_loop
            .run(move |event, window_target| match event {
                Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui
                        .io_mut()
                        .set_delta_time((now - last_frame).as_secs_f32());
                    last_frame = now;
                }

                Event::AboutToWait => {
                    platform
                        .prepare_frame(imgui.io_mut(), &window)
                        .expect("Failed to prepare frame");
                    window.request_redraw();
                }

                Event::WindowEvent {
                    event: ref window_event,
                    ..
                } => {
                    platform.handle_event(imgui.io_mut(), &window, &event);
                    dispatch_window_event(
                        window_event,
                        window_target,
                        app,
                        &mut imgui,
                        &mut platform,
                        &window,
                        &bindings,
                    );
                }

                Event::UserEvent(command) => {
                    app.data.ecs_world.send_boxed_command(command);
                }

                Event::LoopExiting => {
                    unsafe { app.destroy() };
                }

                _ => {}
            })
            .expect("EventLoop error");
    }
}
