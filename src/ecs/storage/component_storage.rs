use std::any::Any;

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
