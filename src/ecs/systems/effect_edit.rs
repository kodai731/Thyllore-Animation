use crate::ecs::storage::Component;
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::scene::SceneOwner;

pub trait EffectPreset: SceneOwner + Clone {
    type Applied: Component;
    fn apply_preset(&mut self, name: &str) -> bool;
    fn applied(name: &str) -> Self::Applied;
}

pub fn apply_effect_update<E: SceneOwner>(world: &mut World, entity: Entity, effect: E) {
    if !world.has_component::<E>(entity) {
        return;
    }

    let (translation, rotation) = effect.placement();
    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
        transform.translation = translation;
        transform.rotation = rotation;
    }
    world.insert_component(entity, effect);
}

pub fn apply_effect_preset<E: EffectPreset>(world: &mut World, target: Entity, name: &str) {
    let Some(mut effect) = world.get_component::<E>(target).cloned() else {
        return;
    };
    if !effect.apply_preset(name) {
        return;
    }

    apply_effect_update(world, target, effect);
    world.insert_component(target, E::applied(name));
}
