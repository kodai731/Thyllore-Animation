use std::any::Any;
use std::collections::HashMap;

use crate::ecs::world::Entity;

pub trait Component: Any + 'static {}
impl<T: Any + 'static> Component for T {}

pub trait ComponentStorage: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove(&mut self, entity: Entity);
    fn contains(&self, entity: Entity) -> bool;
    fn entities(&self) -> Vec<Entity>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn clear(&mut self);
}

pub struct TypedStorage<T: Component> {
    data: HashMap<Entity, T>,
}

impl<T: Component> TypedStorage<T> {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity, component: T) {
        self.data.insert(entity, component);
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.data.get(&entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.data.get_mut(&entity)
    }

    pub fn data(&self) -> &HashMap<Entity, T> {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut HashMap<Entity, T> {
        &mut self.data
    }
}

impl<T: Component> Default for TypedStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Component> ComponentStorage for TypedStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove(&mut self, entity: Entity) {
        self.data.remove(&entity);
    }

    fn contains(&self, entity: Entity) -> bool {
        self.data.contains_key(&entity)
    }

    fn entities(&self) -> Vec<Entity> {
        self.data.keys().copied().collect()
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
