use crate::ecs::storage::Component;
use crate::ecs::world::{Entity, World};

use super::traits::ComponentRef;

impl<T: Component> ComponentRef for T {
    fn get_from_world(world: &World, entity: Entity) -> Option<&Self> {
        world.get_component_ref::<T>(entity)
    }

    fn get_from_world_mut(world: &mut World, entity: Entity) -> Option<&mut Self> {
        world.get_component_ref_mut::<T>(entity)
    }

    fn entities_in_world(world: &World) -> Vec<Entity> {
        world.component_entities::<T>()
    }

    fn exists_in_world(world: &World, entity: Entity) -> bool {
        world.has_component::<T>(entity)
    }
}
