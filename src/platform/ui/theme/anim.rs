use imgui::Ui;

use crate::ecs::resource::{MotionPreference, UiAnimState, UiSettings};
use crate::ecs::World;

pub fn ui_anim(ui: &Ui, world: &World, label: &str, target: f32) -> f32 {
    let id = ui.new_id_str(label).raw();
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
