use std::marker::PhantomData;

use crate::ecs::world::{Entity, World};

use super::traits::{ComponentRef, QueryFilter};

pub struct With<T>(PhantomData<T>);

impl<T: ComponentRef> QueryFilter for With<T> {
    fn matches(world: &World, entity: Entity) -> bool {
        T::exists_in_world(world, entity)
    }
}

pub struct Without<T>(PhantomData<T>);

impl<T: ComponentRef> QueryFilter for Without<T> {
    fn matches(world: &World, entity: Entity) -> bool {
        !T::exists_in_world(world, entity)
    }
}

impl QueryFilter for () {
    fn matches(_world: &World, _entity: Entity) -> bool {
        true
    }
}

impl<A: QueryFilter, B: QueryFilter> QueryFilter for (A, B) {
    fn matches(world: &World, entity: Entity) -> bool {
        A::matches(world, entity) && B::matches(world, entity)
    }
}

impl<A: QueryFilter, B: QueryFilter, C: QueryFilter> QueryFilter for (A, B, C) {
    fn matches(world: &World, entity: Entity) -> bool {
        A::matches(world, entity) && B::matches(world, entity) && C::matches(world, entity)
    }
}
