use anyhow::Context;
use cgmath::{Quaternion, Vector3};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thyllore_scene_core::SceneComponent;

use crate::asset::AssetStorage;
use crate::ecs::component::{EditorDisplay, EntityIcon};
use crate::ecs::world::{Entity, GlobalTransform, Transform, World};

/// Persisted form of one component inside a scene entity; RON keeps the numbers as written.
pub type SceneValue = ron::Value;

/// An owner defines the entity it sits on; an attachment is restored onto an entity an owner
/// has already assembled, so owners are always applied first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneComponentRole {
    Owner,
    Attachment,
}

pub type SceneEntitiesFn = fn(&World) -> Vec<Entity>;
pub type SceneCaptureFn = fn(&World, Entity) -> Option<SceneValue>;
pub type SceneApplyFn =
    fn(&mut World, &mut AssetStorage, Entity, &SceneValue) -> anyhow::Result<()>;

#[derive(Clone, Copy)]
pub struct SceneComponentHook {
    pub type_key: &'static str,
    pub role: SceneComponentRole,
    pub entities: SceneEntitiesFn,
    pub capture: SceneCaptureFn,
    pub apply: SceneApplyFn,
}

/// A component that defines the entity it sits on: the entity is spawned from its name and
/// placement, gets the hierarchy icon and then the component itself.
pub trait SceneOwner: SceneComponent {
    const ICON: EntityIcon;
    fn placement(&self) -> (Vector3<f32>, Quaternion<f32>);
    /// Runs on a component decoded from a scene before it is attached.
    fn prepare_loaded(&mut self) {}
}

impl SceneComponentHook {
    pub const fn owner<C: SceneOwner>() -> Self {
        Self {
            type_key: C::TYPE_KEY,
            role: SceneComponentRole::Owner,
            entities: entities_with::<C>,
            capture: capture_component::<C>,
            apply: attach_loaded_owner::<C>,
        }
    }

    /// A component stored as its serde form and restored by insertion onto the owner's entity.
    pub const fn attachment<C: SceneComponent>() -> Self {
        Self {
            type_key: C::TYPE_KEY,
            role: SceneComponentRole::Attachment,
            entities: entities_with::<C>,
            capture: capture_component::<C>,
            apply: insert_component::<C>,
        }
    }
}

pub fn spawn_scene_owner<C: SceneOwner>(world: &mut World, name: &str, component: C) -> Entity {
    let entity = world.entity().with_name(name).build();
    attach_scene_owner(world, entity, component);
    entity
}

pub fn attach_scene_owner<C: SceneOwner>(world: &mut World, entity: Entity, component: C) {
    let (translation, rotation) = component.placement();
    world.insert_component(
        entity,
        Transform {
            translation,
            rotation,
            ..Default::default()
        },
    );
    world.insert_component(entity, GlobalTransform::new());
    world.insert_component(entity, EditorDisplay::new(C::ICON));
    world.insert_component(entity, component);
}

fn attach_loaded_owner<C: SceneOwner>(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    let mut component: C = decode_component(value)?;
    component.prepare_loaded();
    attach_scene_owner(world, entity, component);
    Ok(())
}

pub fn entities_with<C: 'static>(world: &World) -> Vec<Entity> {
    let mut entities: Vec<Entity> = world.iter_components::<C>().map(|(e, _)| e).collect();
    entities.sort();
    entities
}

pub fn encode_scene_value<T: Serialize>(value: &T) -> anyhow::Result<SceneValue> {
    Ok(ron::from_str(&ron::to_string(value)?)?)
}

pub fn decode_scene_value<T: DeserializeOwned>(value: &SceneValue) -> anyhow::Result<T> {
    Ok(value.clone().into_rust()?)
}

pub fn capture_component<C: SceneComponent>(world: &World, entity: Entity) -> Option<SceneValue> {
    let component = world.get_component::<C>(entity)?;
    match encode_scene_value(&*component) {
        Ok(value) => Some(value),
        Err(error) => {
            log_warn!(
                "Failed to encode scene component {}: {:#}",
                C::TYPE_KEY,
                error
            );
            None
        }
    }
}

pub fn decode_component<C: SceneComponent>(value: &SceneValue) -> anyhow::Result<C> {
    decode_scene_value(value).with_context(|| format!("scene component {}", C::TYPE_KEY))
}

fn insert_component<C: SceneComponent>(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    world.insert_component(entity, decode_component::<C>(value)?);
    Ok(())
}

/// Registry the scene loader and saver consult; owners are listed before attachments.
#[derive(Default)]
pub struct SceneComponentHooks {
    entries: Vec<SceneComponentHook>,
}

impl std::fmt::Debug for SceneComponentHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneComponentHooks")
            .field("type_keys", &self.type_keys())
            .finish()
    }
}

impl SceneComponentHooks {
    pub fn register(&mut self, hook: SceneComponentHook) {
        match self
            .entries
            .iter_mut()
            .find(|entry| entry.type_key == hook.type_key)
        {
            Some(entry) => *entry = hook,
            None => self.entries.push(hook),
        }
    }

    pub fn register_all(&mut self, hooks: &[SceneComponentHook]) {
        for hook in hooks {
            self.register(*hook);
        }
    }

    pub fn type_keys(&self) -> Vec<&'static str> {
        self.ordered().map(|hook| hook.type_key).collect()
    }

    pub fn find(&self, type_key: &str) -> Option<&SceneComponentHook> {
        self.entries.iter().find(|hook| hook.type_key == type_key)
    }

    pub fn owners(&self) -> impl Iterator<Item = &SceneComponentHook> {
        self.with_role(SceneComponentRole::Owner)
    }

    /// Owners in registration order, then attachments in registration order.
    pub fn ordered(&self) -> impl Iterator<Item = &SceneComponentHook> {
        self.owners()
            .chain(self.with_role(SceneComponentRole::Attachment))
    }

    fn with_role(&self, role: SceneComponentRole) -> impl Iterator<Item = &SceneComponentHook> {
        self.entries.iter().filter(move |hook| hook.role == role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_entities(_: &World) -> Vec<Entity> {
        Vec::new()
    }

    fn no_capture(_: &World, _: Entity) -> Option<SceneValue> {
        None
    }

    fn no_apply(
        _: &mut World,
        _: &mut AssetStorage,
        _: Entity,
        _: &SceneValue,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    const fn hook(type_key: &'static str, role: SceneComponentRole) -> SceneComponentHook {
        SceneComponentHook {
            type_key,
            role,
            entities: no_entities,
            capture: no_capture,
            apply: no_apply,
        }
    }

    #[test]
    fn ordered_lists_owners_before_attachments_in_registration_order() {
        let mut hooks = SceneComponentHooks::default();
        hooks.register(hook("clip", SceneComponentRole::Attachment));
        hooks.register(hook("b_owner", SceneComponentRole::Owner));
        hooks.register(hook("a_owner", SceneComponentRole::Owner));
        hooks.register(hook("style", SceneComponentRole::Attachment));

        assert_eq!(hooks.type_keys(), ["b_owner", "a_owner", "clip", "style"]);
    }

    #[test]
    fn registering_the_same_key_twice_replaces_the_entry() {
        let mut hooks = SceneComponentHooks::default();
        hooks.register(hook("x", SceneComponentRole::Attachment));
        hooks.register(hook("x", SceneComponentRole::Owner));

        assert_eq!(hooks.type_keys(), ["x"]);
        assert_eq!(
            hooks.find("x").map(|hook| hook.role),
            Some(SceneComponentRole::Owner)
        );
    }
}
