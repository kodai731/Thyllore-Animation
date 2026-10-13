use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::HierarchyState;
use crate::ecs::storage::Component;
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::scene::SceneOwner;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

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

#[derive(Clone, Debug)]
pub enum EffectEdit<E, R> {
    UpdateEffect { entity: Entity, effect: Box<E> },
    ApplyPreset(String),
    UpdateRenderSettings(R),
}

impl<E: EffectPreset + 'static + std::fmt::Debug, R: 'static + Clone + std::fmt::Debug> UiCommand
    for EffectEdit<E, R>
{
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            EffectEdit::UpdateEffect { entity, effect } => {
                apply_effect_update::<E>(world, entity, *effect);
            }
            EffectEdit::ApplyPreset(name) => {
                let Some(target) = resolve_selected_effect::<E>(world) else {
                    return;
                };
                apply_effect_preset::<E>(world, target, &name);
            }
            EffectEdit::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<R>() {
                    *settings = new_settings;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_selected_effect, EffectEdit, EffectPreset};
    use crate::asset::AssetStorage;
    use crate::ecs::events::UiCommand;
    use crate::ecs::resource::HierarchyState;
    use crate::ecs::world::{Entity, World};
    use crate::scene::test_support::ProbeOwner;
    use crate::vulkanr::resource::graphics_resource::GraphicsResources;

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

    #[derive(Debug, PartialEq)]
    pub struct AppliedProbePreset(String);

    impl EffectPreset for ProbeOwner {
        type Applied = AppliedProbePreset;

        fn apply_preset(&mut self, _name: &str) -> bool {
            self.level += 1.0;
            true
        }

        fn applied(name: &str) -> Self::Applied {
            AppliedProbePreset(name.to_string())
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct ProbeRenderSettings {
        exposure: f32,
    }

    type ProbeEdit = EffectEdit<ProbeOwner, ProbeRenderSettings>;

    fn apply_edit(world: &mut World, edit: ProbeEdit) {
        Box::new(edit).apply(
            world,
            &mut AssetStorage::default(),
            &GraphicsResources::default(),
        );
    }

    #[test]
    fn apply_preset_targets_the_selected_effect() {
        let mut world = World::new();
        let first = spawn_probe(&mut world);
        let second = spawn_probe(&mut world);
        select(&mut world, Some(second));

        apply_edit(&mut world, ProbeEdit::ApplyPreset("calm".to_string()));

        assert_eq!(
            world.get_component::<ProbeOwner>(second).unwrap().level,
            1.0
        );
        assert_eq!(
            world.get_component::<AppliedProbePreset>(second),
            Some(&AppliedProbePreset("calm".to_string()))
        );
        assert_eq!(world.get_component::<ProbeOwner>(first).unwrap().level, 0.0);
        assert!(world.get_component::<AppliedProbePreset>(first).is_none());
    }

    #[test]
    fn update_render_settings_replaces_the_resource() {
        let mut world = World::new();
        world.insert_resource(ProbeRenderSettings { exposure: 0.0 });

        apply_edit(
            &mut world,
            ProbeEdit::UpdateRenderSettings(ProbeRenderSettings { exposure: 2.5 }),
        );

        assert_eq!(
            *world.resource::<ProbeRenderSettings>(),
            ProbeRenderSettings { exposure: 2.5 }
        );
    }
}
