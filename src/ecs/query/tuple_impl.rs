use std::collections::HashSet;

use crate::ecs::world::{Entity, World};

use super::traits::{ComponentRef, QueryData};

impl<T: ComponentRef> QueryData for &T {
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

impl<T: ComponentRef> QueryData for Option<&T> {
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

macro_rules! impl_tuple_query {
    ($($T:ident),+) => {
        impl<$($T: QueryData),+> QueryData for ($($T,)+) {
            type Item<'a> = ($($T::Item<'a>,)+);

            fn get<'a>(world: &'a World, entity: Entity) -> Option<Self::Item<'a>> {
                Some(($($T::get(world, entity)?,)+))
            }

            fn entities(world: &World) -> Vec<Entity> {
                let sets: Vec<HashSet<Entity>> = vec![
                    $($T::entities(world).into_iter().collect()),+
                ];

                let required_sets: Vec<_> = {
                    let mut result = Vec::new();
                    let mut idx = 0;
                    $(
                        if $T::is_required() {
                            result.push(sets[idx].clone());
                        }
                        #[allow(unused_assignments)]
                        { idx += 1; }
                    )+
                    result
                };

                if required_sets.is_empty() {
                    sets.into_iter()
                        .reduce(|a, b| a.union(&b).copied().collect())
                        .unwrap_or_default()
                        .into_iter()
                        .collect()
                } else {
                    required_sets
                        .into_iter()
                        .reduce(|a, b| a.intersection(&b).copied().collect())
                        .unwrap_or_default()
                        .into_iter()
                        .collect()
                }
            }

            fn is_required() -> bool {
                false $(|| $T::is_required())+
            }
        }
    };
}

impl_tuple_query!(A);
impl_tuple_query!(A, B);
impl_tuple_query!(A, B, C);
impl_tuple_query!(A, B, C, D);
