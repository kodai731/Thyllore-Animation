mod bone_label;
mod context_menu;
mod dopesheet;
mod draw;
mod interaction;
mod keyboard;
mod numeric_input;
mod track_list;
mod tween;
mod view;
mod window;

crate::ui_window!(
    "curve_editor",
    Floating,
    0,
    window::build_curve_editor_window
);
