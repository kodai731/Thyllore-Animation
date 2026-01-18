use std::marker::PhantomData;

use crate::ecs::world::{Entity, World};

use super::iterator::{QueryFilteredIter, QueryIter};
use super::traits::{QueryData, QueryFilter};

pub struct Query<'w, Q: QueryData> {
    world: &'w World,
    _marker: PhantomData<Q>,
}

impl<'w, Q: QueryData> Query<'w, Q> {
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'w, Q> {
        QueryIter::new(self.world)
    }
}

impl<'w, Q: QueryData> IntoIterator for Query<'w, Q> {
    type Item = (Entity, Q::Item<'w>);
    type IntoIter = QueryIter<'w, Q>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter::new(self.world)
    }
}

pub struct QueryFiltered<'w, Q: QueryData, F: QueryFilter> {
    world: &'w World,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, Q: QueryData, F: QueryFilter> QueryFiltered<'w, Q, F> {
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryFilteredIter<'w, Q, F> {
        QueryFilteredIter::new(self.world)
    }
}

impl<'w, Q: QueryData, F: QueryFilter> IntoIterator for QueryFiltered<'w, Q, F> {
    type Item = (Entity, Q::Item<'w>);
    type IntoIter = QueryFilteredIter<'w, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryFilteredIter::new(self.world)
    }
}
