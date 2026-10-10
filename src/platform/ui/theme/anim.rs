use imgui::Ui;

use crate::ecs::resource::{MotionPreference, UiAnimState, UiSettings};
use crate::ecs::World;

#[inline]
fn hash_label(label: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in label.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

pub fn ui_anim(ui: &Ui, world: &World, label: &str, target: f32) -> f32 {
    let id = hash_label(label);
    let dt = ui.io().delta_time;
    let motion = match world.get_resource::<UiSettings>() {
        Some(settings) => settings.reduced_motion,
        None => MotionPreference::Full,
    };

    let Some(mut state) = world.get_resource_mut::<UiAnimState>() else {
        return target;
    };
    state.animate(id, target, dt, motion)
}
