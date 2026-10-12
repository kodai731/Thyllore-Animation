use std::time::Instant;

use imgui::Context;
use imgui_winit_support::WinitPlatform;
use winit::event::Event;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use super::events::dispatch_window_event;
use super::key_bindings::{default_bindings, KeyBinding};
use crate::app::App;
use crate::hooks::external_command::{ExternalCommand, ExternalCommandSender};
use crate::platform::System;

struct FrameDriver {
    window: Window,
    imgui: Context,
    platform: WinitPlatform,
    last_frame: Instant,
    bindings: Vec<KeyBinding>,
}

impl FrameDriver {
    fn handle(
        &mut self,
        event: Event<ExternalCommand>,
        window_target: &EventLoopWindowTarget<ExternalCommand>,
        app: &mut App,
    ) {
        match event {
            Event::NewEvents(_) => self.advance_frame_clock(),
            Event::AboutToWait => self.prepare_frame(),
            Event::WindowEvent { .. } => self.forward_window_event(&event, window_target, app),
            Event::UserEvent(command) => app.data.ecs_world.send_boxed_command(command),
            Event::LoopExiting => unsafe { app.destroy() },
            _ => {}
        }
    }

    fn advance_frame_clock(&mut self) {
        let now = Instant::now();
        self.imgui
            .io_mut()
            .set_delta_time((now - self.last_frame).as_secs_f32());
        self.last_frame = now;
    }

    fn prepare_frame(&mut self) {
        self.platform
            .prepare_frame(self.imgui.io_mut(), &self.window)
            .expect("Failed to prepare frame");
        self.window.request_redraw();
    }

    fn forward_window_event(
        &mut self,
        event: &Event<ExternalCommand>,
        window_target: &EventLoopWindowTarget<ExternalCommand>,
        app: &mut App,
    ) {
        let Event::WindowEvent {
            event: window_event,
            ..
        } = event
        else {
            return;
        };
        self.platform
            .handle_event(self.imgui.io_mut(), &self.window, event);
        dispatch_window_event(
            window_event,
            window_target,
            app,
            &mut self.imgui,
            &mut self.platform,
            &self.window,
            &self.bindings,
        );
    }
}

impl System {
    pub fn external_command_sender(&self) -> ExternalCommandSender {
        ExternalCommandSender::new(self.event_loop.create_proxy())
    }

    pub fn main_loop(self, app: &mut App) {
        let System {
            event_loop,
            window,
            imgui,
            platform,
            ui_fonts: _ui_fonts,
        } = self;
        let mut driver = FrameDriver {
            window,
            imgui,
            platform,
            last_frame: Instant::now(),
            bindings: default_bindings(),
        };

        event_loop
            .run(move |event, window_target| driver.handle(event, window_target, app))
            .expect("EventLoop error");
    }
}
