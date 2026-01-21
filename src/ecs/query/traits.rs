use crate::ecs::world::{Entity, World};

pub trait ComponentRef: Sized + 'static {
    fn get_from_world(world: &World, entity: Entity) -> Option<&Self>;
    fn get_from_world_mut(world: &mut World, entity: Entity) -> Option<&mut Self>;
    fn entities_in_world(world: &World) -> Vec<Entity>;
    fn exists_in_world(world: &World, entity: Entity) -> bool;
}

pub trait QueryData {
    type Item<'a>;
    fn get<'a>(world: &'a World, entity: Entity) -> Option<Self::Item<'a>>;
    fn entities(world: &World) -> Vec<Entity>;
    fn is_required() -> bool;
}

pub trait QueryFilter {
    fn matches(world: &World, entity: Entity) -> bool;
}
