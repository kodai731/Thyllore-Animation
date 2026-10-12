use std::ffi::CStr;

use imgui::MouseButton;

use crate::ecs::resource::{InjectedInput, UiFocusState};
use crate::ecs::World;

/// Releases last frame's injected chords and presses the queued ones before imgui starts the frame.
pub(super) fn feed_injected_keys(io: &mut imgui::Io, world: &World) {
    let mut injected = world.resource_mut::<InjectedInput>();

    for press in injected.held_presses.drain(..) {
        for key in press.keys_in_press_order().into_iter().rev() {
            io.add_key_event(key, false);
        }
    }

    let presses = std::mem::take(&mut injected.pending_presses);
    for press in &presses {
        for key in press.keys_in_press_order() {
            io.add_key_event(key, true);
        }
    }
    injected.held_presses = presses;

    feed_injected_mouse(io, &mut injected);
}

/// A click presses the left button at its position; the release follows on the next frame so
/// imgui sees a full press, the same way `feed_injected_keys` holds its chords.
fn feed_injected_mouse(io: &mut imgui::Io, injected: &mut InjectedInput) {
    if injected.click_held {
        io.add_mouse_button_event(MouseButton::Left, false);
        injected.click_held = false;
    }
    if let Some(position) = injected.pending_mouse_move.take() {
        io.add_mouse_pos_event(position);
    }
    if let Some(position) = injected.pending_click.take() {
        io.add_mouse_pos_event(position);
        io.add_mouse_button_event(MouseButton::Left, true);
        injected.click_held = true;
    }
}

pub(super) fn apply_injected_window_focus(ui: &imgui::Ui, world: &World) {
    let title = world
        .resource_mut::<InjectedInput>()
        .pending_window_focus
        .take();
    if let Some(title) = title {
        ui.set_window_focus(Some(&title));
    }
}

/// Records imgui's navigation window so a headless run can read back where its focus request landed.
pub(super) fn record_focused_window(_: &imgui::Ui, world: &World) {
    let focused_window = unsafe {
        let nav_window = (*imgui::sys::igGetCurrentContext()).NavWindow;
        (!nav_window.is_null()).then(|| {
            CStr::from_ptr((*nav_window).Name)
                .to_string_lossy()
                .into_owned()
        })
    };
    world.resource_mut::<UiFocusState>().focused_window = focused_window;
}
