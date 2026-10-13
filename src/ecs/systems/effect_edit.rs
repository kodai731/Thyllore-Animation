use crate::ecs::resource::HierarchyState;
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

pub fn resolve_selected_effect<C: Component>(world: &World) -> Option<Entity> {
    let selected = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity);

    if let Some(entity) = selected {
        if world.has_component::<C>(entity) {
            return Some(entity);
        }
    }

    world.entities_with::<C>().first().copied()
}

#[cfg(test)]
mod tests {
    use super::resolve_selected_effect;
    use crate::ecs::resource::HierarchyState;
    use crate::ecs::world::{Entity, World};
    use crate::scene::test_support::ProbeOwner;

    fn spawn_probe(world: &mut World) -> Entity {
        let entity = world.spawn();
        world.insert_component(entity, ProbeOwner::default());
        entity
    }

    fn select(world: &mut World, entity: Option<Entity>) {
        world.insert_resource(HierarchyState {
            selected_entity: entity,
            ..Default::default()
        });
    }

    #[test]
    fn selected_entity_with_the_component_wins_in_resolve_selected_effect() {
        let mut world = World::new();
        spawn_probe(&mut world);
        let second = spawn_probe(&mut world);
        select(&mut world, Some(second));

        assert_eq!(resolve_selected_effect::<ProbeOwner>(&world), Some(second));
    }

    #[test]
    fn selected_entity_without_the_component_falls_back_in_resolve_selected_effect() {
        let mut world = World::new();
        let first = spawn_probe(&mut world);
        spawn_probe(&mut world);
        let unrelated = world.spawn();
        select(&mut world, Some(unrelated));

        assert_eq!(resolve_selected_effect::<ProbeOwner>(&world), Some(first));
    }

    #[test]
    fn no_selection_falls_back_in_resolve_selected_effect() {
        let mut world = World::new();
        let first = spawn_probe(&mut world);
        spawn_probe(&mut world);
        select(&mut world, None);

        assert_eq!(resolve_selected_effect::<ProbeOwner>(&world), Some(first));
    }
}
