use std::any::Any;

use crate::ecs::world::Entity;

use super::component_storage::{Component, ComponentStorage};
use super::sparse_set::SparseSet;

pub struct TypedStorage<T: Component> {
    data: SparseSet<T>,
}

impl<T: Component> Default for TypedStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Component> TypedStorage<T> {
    pub fn new() -> Self {
        Self {
            data: SparseSet::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: SparseSet::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, entity: Entity, component: T) {
        self.data.insert(entity, component);
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.data.get(entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.data.get_mut(entity)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.data.contains(entity)
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.data.remove(entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.data.iter_mut()
    }

    pub fn dense(&self) -> &[T] {
        self.data.dense()
    }

    pub fn dense_mut(&mut self) -> &mut [T] {
        self.data.dense_mut()
    }

    pub fn entities_slice(&self) -> &[Entity] {
        self.data.entities()
    }
}

impl<T: Component> ComponentStorage for TypedStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn contains(&self, entity: Entity) -> bool {
        self.data.contains(entity)
    }

    fn remove(&mut self, entity: Entity) {
        self.data.remove(entity);
    }

    fn entities(&self) -> Vec<Entity> {
        self.data.entities().to_vec()
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn clear(&mut self) {
        self.data.clear();
    }
}
