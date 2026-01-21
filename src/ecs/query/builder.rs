use std::collections::HashSet;
use std::marker::PhantomData;

use crate::ecs::world::{Entity, World};

use super::fetch::{Fetch, FetchOptional, FilterWith};
use super::hlist::{HCons, HList, HNil};
use super::iterator::QueryIter;
use super::query::Query;
use super::traits::{ComponentRef, QueryData};

impl QueryData for HNil {
    type Item<'a> = ();

    fn get<'a>(_world: &'a World, _entity: Entity) -> Option<Self::Item<'a>> {
        Some(())
    }

    fn entities(world: &World) -> Vec<Entity> {
        world.component_entities::<crate::ecs::world::Transform>()
    }

    fn is_required() -> bool {
        false
    }
}

impl<H: QueryData, T: QueryData + HList> QueryData for HCons<H, T> {
    type Item<'a> = HCons<H::Item<'a>, T::Item<'a>>;

    fn get<'a>(world: &'a World, entity: Entity) -> Option<Self::Item<'a>> {
        let head = H::get(world, entity)?;
        let tail = T::get(world, entity)?;
        Some(HCons::new(head, tail))
    }

    fn entities(world: &World) -> Vec<Entity> {
        let head_entities: HashSet<_> = H::entities(world).into_iter().collect();
        let tail_entities: HashSet<_> = T::entities(world).into_iter().collect();

        if H::is_required() && T::is_required() {
            head_entities
                .intersection(&tail_entities)
                .copied()
                .collect()
        } else if H::is_required() {
            head_entities.into_iter().collect()
        } else if T::is_required() {
            tail_entities.into_iter().collect()
        } else {
            head_entities.union(&tail_entities).copied().collect()
        }
    }

    fn is_required() -> bool {
        H::is_required() || T::is_required()
    }
}

pub struct QueryBuilder<'w, Q: QueryData> {
    world: &'w World,
    _marker: PhantomData<Q>,
}

impl<'w> QueryBuilder<'w, HNil> {
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }
}

impl<'w, Q: QueryData + HList> QueryBuilder<'w, Q> {
    pub fn with<T: ComponentRef>(self) -> QueryBuilder<'w, HCons<Fetch<T>, Q>> {
        QueryBuilder {
            world: self.world,
            _marker: PhantomData,
        }
    }

    pub fn with_optional<T: ComponentRef>(self) -> QueryBuilder<'w, HCons<FetchOptional<T>, Q>> {
        QueryBuilder {
            world: self.world,
            _marker: PhantomData,
        }
    }

    pub fn filter<T: ComponentRef>(self) -> QueryBuilder<'w, HCons<FilterWith<T>, Q>> {
        QueryBuilder {
            world: self.world,
            _marker: PhantomData,
        }
    }

    pub fn build(self) -> Query<'w, Q> {
        Query::new(self.world)
    }

    pub fn iter(self) -> QueryIter<'w, Q> {
        QueryIter::new(self.world)
    }
}
