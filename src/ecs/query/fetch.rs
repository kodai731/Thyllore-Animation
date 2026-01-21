use std::marker::PhantomData;

use crate::ecs::world::{Entity, World};

use super::traits::{ComponentRef, QueryData};

pub struct Fetch<T>(PhantomData<T>);

impl<T: ComponentRef> QueryData for Fetch<T> {
    type Item<'a> = &'a T;

    fn get<'a>(world: &'a World, entity: Entity) -> Option<Self::Item<'a>> {
        T::get_from_world(world, entity)
    }

    fn entities(world: &World) -> Vec<Entity> {
        T::entities_in_world(world)
    }

    fn is_required() -> bool {
        true
    }
}

pub struct FetchOptional<T>(PhantomData<T>);

impl<T: ComponentRef> QueryData for FetchOptional<T> {
    type Item<'a> = Option<&'a T>;

    fn get<'a>(world: &'a World, entity: Entity) -> Option<Self::Item<'a>> {
        Some(T::get_from_world(world, entity))
    }

    fn entities(world: &World) -> Vec<Entity> {
        T::entities_in_world(world)
    }

    fn is_required() -> bool {
        false
    }
}

pub struct FilterWith<T>(PhantomData<T>);

impl<T: ComponentRef> QueryData for FilterWith<T> {
    type Item<'a> = ();

    fn get<'a>(world: &'a World, entity: Entity) -> Option<Self::Item<'a>> {
        if T::exists_in_world(world, entity) {
            Some(())
        } else {
            None
        }
    }

    fn entities(world: &World) -> Vec<Entity> {
        T::entities_in_world(world)
    }

    fn is_required() -> bool {
        true
    }
}
