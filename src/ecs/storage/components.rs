use std::any::TypeId;
use std::collections::HashMap;

use crate::ecs::world::Entity;

use super::component_storage::{Component, ComponentStorage, TypedStorage};

pub struct Components {
    storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
}

impl std::fmt::Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Components")
            .field("storage_count", &self.storages.len())
            .finish()
    }
}

impl Default for Components {
    fn default() -> Self {
        Self::new()
    }
}

impl Components {
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
        }
    }

    pub fn register<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        if !self.storages.contains_key(&type_id) {
            self.storages
                .insert(type_id, Box::new(TypedStorage::<T>::new()));
        }
    }

    pub fn is_registered<T: Component>(&self) -> bool {
        self.storages.contains_key(&TypeId::of::<T>())
    }

    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.storages.get_mut(&type_id) {
            let typed = storage
                .as_any_mut()
                .downcast_mut::<TypedStorage<T>>()
                .expect("Type mismatch in component storage");
            typed.insert(entity, component);
        } else {
            panic!(
                "Component type {} not registered. Call register::<{}>() first.",
                std::any::type_name::<T>(),
                std::any::type_name::<T>()
            );
        }
    }

    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.storages.get(&type_id).and_then(|storage| {
            let typed = storage
                .as_any()
                .downcast_ref::<TypedStorage<T>>()
                .expect("Type mismatch in component storage");
            typed.get(entity)
        })
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id).and_then(|storage| {
            let typed = storage
                .as_any_mut()
                .downcast_mut::<TypedStorage<T>>()
                .expect("Type mismatch in component storage");
            typed.get_mut(entity)
        })
    }

    pub fn contains<T: Component>(&self, entity: Entity) -> bool {
        let type_id = TypeId::of::<T>();
        self.storages
            .get(&type_id)
            .map(|storage| storage.contains(entity))
            .unwrap_or(false)
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.storages.get_mut(&type_id) {
            storage.remove(entity);
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
    }

    pub fn entities<T: Component>(&self) -> Vec<Entity> {
        let type_id = TypeId::of::<T>();
        self.storages
            .get(&type_id)
            .map(|storage| storage.entities())
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        for storage in self.storages.values_mut() {
            storage.clear();
        }
    }

    pub fn storage_count(&self) -> usize {
        self.storages.len()
    }
}
