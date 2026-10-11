mod bone_label;
mod context_menu;
mod dopesheet;
mod drag;
mod draw;
mod hit_test;
mod interaction;
mod keyboard;
mod numeric_input;
mod selection;
#[cfg(test)]
mod test_fixtures;
mod track_list;
mod tween;
mod view;
mod window;

pub(crate) use keyboard::CURVE_EDITOR_KEY_BINDINGS;
pub(crate) use window::build_curve_editor_tab;
