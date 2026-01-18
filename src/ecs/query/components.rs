use crate::ecs::world::{
    AnimationState, BillboardBehavior, Children, Entity, GlobalTransform, LineRendering,
    MaterialRef, MeshRef, Name, NodeRef, Parent, SkeletonRef, SkinRef, Transform, Visible, World,
};

use super::traits::ComponentRef;

macro_rules! impl_component_ref {
    ($comp:ty, $field:ident) => {
        impl ComponentRef for $comp {
            fn get_from_world(world: &World, entity: Entity) -> Option<&Self> {
                world.$field.get(&entity)
            }

            fn get_from_world_mut(world: &mut World, entity: Entity) -> Option<&mut Self> {
                world.$field.get_mut(&entity)
            }

            fn entities_in_world(world: &World) -> Vec<Entity> {
                world.$field.keys().copied().collect()
            }

            fn exists_in_world(world: &World, entity: Entity) -> bool {
                world.$field.contains_key(&entity)
            }
        }
    };
}

impl_component_ref!(Name, names);
impl_component_ref!(Transform, transforms);
impl_component_ref!(GlobalTransform, global_transforms);
impl_component_ref!(Visible, visibles);
impl_component_ref!(Parent, parents);
impl_component_ref!(Children, children);
impl_component_ref!(MeshRef, mesh_refs);
impl_component_ref!(MaterialRef, material_refs);
impl_component_ref!(SkeletonRef, skeleton_refs);
impl_component_ref!(AnimationState, animation_states);
impl_component_ref!(LineRendering, line_renderings);
impl_component_ref!(BillboardBehavior, billboard_behaviors);
impl_component_ref!(NodeRef, node_refs);
impl_component_ref!(SkinRef, skin_refs);
