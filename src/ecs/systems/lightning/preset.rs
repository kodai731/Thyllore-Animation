use super::resolve_selected_lightning;
use crate::ecs::component::LightningEffect;
use crate::ecs::world::World;

pub const LIGHTNING_PRESET_NAMES: [&str; 1] = ["bolt"];

pub fn build_lightning_preset(name: &str) -> Option<LightningEffect> {
    match name {
        "bolt" => Some(LightningEffect::default()),
        _ => None,
    }
}

pub fn apply_lightning_preset_to_selected(world: &mut World, name: &str) {
    let Some(target) = resolve_selected_lightning(world) else {
        return;
    };
    let Some(preset) = build_lightning_preset(name) else {
        return;
    };
    world.insert_component(target, preset);
}
