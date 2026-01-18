use std::marker::PhantomData;

use crate::ecs::world::{Entity, World};

use super::traits::{QueryData, QueryFilter};

pub struct QueryIter<'w, Q: QueryData> {
    world: &'w World,
    entities: std::vec::IntoIter<Entity>,
    _marker: PhantomData<Q>,
}

impl<'w, Q: QueryData> QueryIter<'w, Q> {
    pub fn new(world: &'w World) -> Self {
        let entities = Q::entities(world).into_iter();
        Self {
            world,
            entities,
            _marker: PhantomData,
        }
    }
}

impl<'w, Q: QueryData> Iterator for QueryIter<'w, Q> {
    type Item = (Entity, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.entities.next()?;
            if let Some(item) = Q::get(self.world, entity) {
                return Some((entity, item));
            }
        }
    }
}

pub struct QueryFilteredIter<'w, Q: QueryData, F: QueryFilter> {
    world: &'w World,
    entities: std::vec::IntoIter<Entity>,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, Q: QueryData, F: QueryFilter> QueryFilteredIter<'w, Q, F> {
    pub fn new(world: &'w World) -> Self {
        let entities = Q::entities(world).into_iter();
        Self {
            world,
            entities,
            _marker: PhantomData,
        }
    }
}

impl<'w, Q: QueryData, F: QueryFilter> Iterator for QueryFilteredIter<'w, Q, F> {
    type Item = (Entity, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.entities.next()?;
            if !F::matches(self.world, entity) {
                continue;
            }
            if let Some(item) = Q::get(self.world, entity) {
                return Some((entity, item));
            }
        }
    }
}
