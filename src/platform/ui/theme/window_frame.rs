use imgui::{Condition, Ui, Window};

use super::elevation::{draw_raised_window_frame, push_raised_window_style};

#[derive(Clone, Copy, Debug)]
pub enum WindowKind {
    Docked { position: [f32; 2], size: [f32; 2] },
    Overlay { position: [f32; 2], size: [f32; 2] },
}

#[derive(Clone, Copy, Debug)]
pub enum FloatingPlacement {
    Centered,
    Sized([f32; 2]),
}

pub fn window_frame<'ui>(ui: &'ui Ui, name: &'ui str, kind: WindowKind) -> Window<'ui> {
    match kind {
        WindowKind::Docked { position, size } => ui
            .window(name)
            .position(position, Condition::Always)
            .size(size, Condition::Always)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .bring_to_front_on_focus(false),
        WindowKind::Overlay { position, size } => ui
            .window(name)
            .position(position, Condition::Always)
            .size(size, Condition::Always)
            .no_decoration()
            .focus_on_appearing(false)
            .save_settings(false),
    }
}

pub fn show_floating_window(
    ui: &Ui,
    name: &str,
    placement: FloatingPlacement,
    is_open: &mut bool,
    contents: impl FnOnce(),
) {
    let _raised = push_raised_window_style(ui);
    let mut window = ui.window(name).collapsible(false).opened(is_open);
    match placement {
        FloatingPlacement::Centered => {
            let display_size = ui.io().display_size();
            window = window
                .position(
                    [display_size[0] * 0.5, display_size[1] * 0.5],
                    Condition::Always,
                )
                .position_pivot([0.5, 0.5])
                .always_auto_resize(true);
        }
        FloatingPlacement::Sized(size) => {
            window = window.size(size, Condition::FirstUseEver);
        }
    }
    window.build(|| {
        draw_raised_window_frame(ui);
        contents();
    });
}
