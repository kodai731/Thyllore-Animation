use imgui::MouseButton;

use crate::app::App;
use crate::platform::key_bindings::{dispatch_keyboard_shortcut, ModifierKeys};

use crate::ecs::resource::{CameraFlyInput, KeyboardModifiers, MouseInput};
use crate::ecs::UIEventQueue;

pub(crate) fn update_mouse_input(world: &crate::ecs::World, ui: &imgui::Ui) {
    let io = ui.io();
    let mut mouse = world.resource_mut::<MouseInput>();
    mouse.position = io.mouse_pos;
    mouse.left_pressed = ui.is_mouse_down(MouseButton::Left);
    mouse.right_pressed = ui.is_mouse_down(MouseButton::Right);
    mouse.middle_pressed = ui.is_mouse_down(MouseButton::Middle);
    drop(mouse);

    let mut modifiers = world.resource_mut::<KeyboardModifiers>();
    modifiers.ctrl = io.key_ctrl;
    modifiers.shift = io.key_shift;
    modifiers.alt = io.key_alt;
    drop(modifiers);

    update_camera_fly_input(world, ui);
}

fn update_camera_fly_input(world: &crate::ecs::World, ui: &imgui::Ui) {
    let io = ui.io();
    let mut fly = world.resource_mut::<CameraFlyInput>();
    fly.delta_seconds = io.delta_time;

    if io.want_text_input {
        fly.forward = 0.0;
        fly.right = 0.0;
        fly.up = 0.0;
        fly.boost = false;
        return;
    }

    let axis = |negative: bool, positive: bool| (positive as i32 - negative as i32) as f32;
    fly.forward = axis(ui.is_key_down(imgui::Key::S), ui.is_key_down(imgui::Key::W));
    fly.right = axis(ui.is_key_down(imgui::Key::A), ui.is_key_down(imgui::Key::D));
    fly.up = axis(ui.is_key_down(imgui::Key::Q), ui.is_key_down(imgui::Key::E));
    fly.boost = io.key_shift;
}

pub(crate) fn dispatch_keyboard_input(
    app: &mut App,
    event: &winit::event::KeyEvent,
    imgui: &imgui::Context,
    bindings: &[crate::platform::key_bindings::KeyBinding],
) {
    let modifiers_res = app.data.ecs_world.resource::<KeyboardModifiers>();
    let modifiers = ModifierKeys {
        ctrl: modifiers_res.ctrl,
        shift: modifiers_res.shift,
    };
    drop(modifiers_res);

    let camera_fly_active = app.data.ecs_world.resource::<MouseInput>().right_pressed;

    if let Some(ui_event) = dispatch_keyboard_shortcut(
        &event.logical_key,
        modifiers,
        imgui.io().want_capture_keyboard || camera_fly_active,
        bindings,
    ) {
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        ui_events.send(ui_event);
    }
}
