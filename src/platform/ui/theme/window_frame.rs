use imgui::{Condition, Ui, Window};

#[derive(Clone, Copy, Debug)]
pub enum WindowKind {
    Docked { position: [f32; 2], size: [f32; 2] },
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
    }
}
