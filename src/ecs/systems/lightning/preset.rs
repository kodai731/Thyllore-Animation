use super::resolve_selected_lightning;
use crate::ecs::component::LightningEffect;
use crate::ecs::world::World;

pub fn apply_lightning_preset_to_selected(world: &mut World, name: &str) {
    let Some(target) = resolve_selected_lightning(world) else {
        return;
    };
    let Some(mut effect) = world
        .get_component::<LightningEffect>(target)
        .map(|e| e.clone())
    else {
        return;
    };
    if thyllore_effect_core::apply_lightning_preset(&mut effect, name) {
        world.insert_component(target, effect);
    }
}
