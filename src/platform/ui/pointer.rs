//! The only UI module that reads the raw imgui mouse state; `tests/ui_raw_pointer_reads.rs` enforces it.

use imgui::MouseButton;

use crate::ecs::resource::{UiPointerOwner, UiPointerOwnerId};
use crate::ecs::world::World;

#[derive(Clone, Copy, Debug, Default)]
pub struct UiPointer {
    pub pos: [f32; 2],
    pub delta: [f32; 2],
    pub wheel: f32,
    down: [bool; MOUSE_BUTTONS.len()],
    clicked: [bool; MOUSE_BUTTONS.len()],
    double_clicked: [bool; MOUSE_BUTTONS.len()],
    released: [bool; MOUSE_BUTTONS.len()],
}

const MOUSE_BUTTONS: [MouseButton; 5] = [
    MouseButton::Left,
    MouseButton::Right,
    MouseButton::Middle,
    MouseButton::Extra1,
    MouseButton::Extra2,
];

impl UiPointer {
    pub fn is_down(&self, button: MouseButton) -> bool {
        self.down[button as usize]
    }

    pub fn is_clicked(&self, button: MouseButton) -> bool {
        self.clicked[button as usize]
    }

    pub fn is_double_clicked(&self, button: MouseButton) -> bool {
        self.double_clicked[button as usize]
    }

    pub fn is_released(&self, button: MouseButton) -> bool {
        self.released[button as usize]
    }
}

/// Where the press must land for a handler to take the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerRegion {
    /// The current window is the topmost one under the cursor and no widget is being dragged.
    Window,
    /// Same as `Window`, counting the window's child windows as part of it.
    WindowWithChildren,
    /// The last submitted item (e.g. an `invisible_button` covering a canvas) is hovered.
    LastItem,
    /// Anywhere on screen; for handles drawn outside every window, such as panel splitters.
    Screen,
}

pub fn read_ui_pointer(ui: &imgui::Ui) -> UiPointer {
    let io = ui.io();
    let mut pointer = UiPointer {
        pos: io.mouse_pos(),
        delta: io.mouse_delta(),
        wheel: io.mouse_wheel(),
        ..UiPointer::default()
    };
    for button in MOUSE_BUTTONS {
        let index = button as usize;
        pointer.down[index] = ui.is_mouse_down(button);
        pointer.clicked[index] = ui.is_mouse_clicked(button);
        pointer.double_clicked[index] = ui.is_mouse_double_clicked(button);
        pointer.released[index] = ui.is_mouse_released(button);
    }
    pointer
}

/// True when `owner` already holds the pointer, or when it is free and `region` is under the cursor.
pub fn ui_pointer_available(
    ui: &imgui::Ui,
    world: &World,
    owner: UiPointerOwnerId,
    region: PointerRegion,
) -> bool {
    let pointer_owner = *world.resource::<UiPointerOwner>();
    pointer_owner.is_held_by(owner)
        || (pointer_owner.is_free_for(owner) && is_region_hovered(ui, region))
}

/// Takes the pointer for `owner` when it is available; call it at the moment a drag or click starts.
pub fn ui_pointer_begin(
    ui: &imgui::Ui,
    world: &World,
    owner: UiPointerOwnerId,
    region: PointerRegion,
) -> bool {
    ui_pointer_available(ui, world, owner, region)
        && world.resource_mut::<UiPointerOwner>().try_claim(owner)
}

pub fn is_last_item_double_clicked(ui: &imgui::Ui) -> bool {
    ui.is_item_hovered() && ui.is_mouse_double_clicked(MouseButton::Left)
}

/// Run once per frame before the windows are built.
pub fn release_idle_ui_pointer(ui: &imgui::Ui, world: &World) {
    let any_button_down = MOUSE_BUTTONS.iter().any(|button| ui.is_mouse_down(*button));
    world
        .resource_mut::<UiPointerOwner>()
        .release_when_buttons_up(any_button_down);
}

fn is_region_hovered(ui: &imgui::Ui, region: PointerRegion) -> bool {
    match region {
        PointerRegion::Window => ui.is_window_hovered() && !ui.is_any_item_active(),
        PointerRegion::WindowWithChildren => {
            ui.is_window_hovered_with_flags(imgui::WindowHoveredFlags::CHILD_WINDOWS)
                && !ui.is_any_item_active()
        }
        PointerRegion::LastItem => ui.is_item_hovered(),
        PointerRegion::Screen => true,
    }
}
