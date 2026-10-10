mod bone_label;
mod context_menu;
mod draw;
mod interaction;
mod keyboard;
mod track_list;
mod view;
mod window;

use bone_label::{format_bone_label, order_bone_ids_by_role};
use context_menu::*;
use draw::*;
use interaction::*;
use keyboard::*;
use track_list::*;
use view::*;
use window::*;

crate::ui_window!("curve_editor", Floating, 0, build_curve_editor_window);
