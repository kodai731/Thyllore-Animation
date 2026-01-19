use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::animation::AnimationClipId;
use crate::asset::AssetId;
use crate::ecs::component::{Animated, MeshHandle, Model, Skinned};

pub trait Resource: Any + 'static {}
impl<T: Any + 'static> Resource for T {}

pub struct Resources {
    data: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl std::fmt::Debug for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resources")
            .field("count", &self.data.len())
            .finish()
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ResRef<'a, T: 'static>(Ref<'a, Box<dyn Any>>, std::marker::PhantomData<T>);
pub struct ResMut<'a, T: 'static>(RefMut<'a, Box<dyn Any>>, std::marker::PhantomData<T>);

impl<'a, T: 'static> std::ops::Deref for ResRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.downcast_ref::<T>().unwrap()
    }
}

impl<'a, T: 'static> std::ops::Deref for ResMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.downcast_ref::<T>().unwrap()
    }
}

impl<'a, T: 'static> std::ops::DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.downcast_mut::<T>().unwrap()
    }
}

impl Resources {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert<R: Resource>(&mut self, resource: R) {
        let type_id = TypeId::of::<R>();
        self.data.insert(type_id, RefCell::new(Box::new(resource)));
    }

    pub fn get<R: Resource>(&self) -> Option<ResRef<R>> {
        let type_id = TypeId::of::<R>();
        self.data
            .get(&type_id)
            .map(|cell| ResRef(cell.borrow(), std::marker::PhantomData))
    }

    pub fn get_mut<R: Resource>(&self) -> Option<ResMut<R>> {
        let type_id = TypeId::of::<R>();
        self.data
            .get(&type_id)
            .map(|cell| ResMut(cell.borrow_mut(), std::marker::PhantomData))
    }

    pub fn remove<R: Resource>(&mut self) -> Option<R> {
        let type_id = TypeId::of::<R>();
        self.data
            .remove(&type_id)
            .and_then(|cell| cell.into_inner().downcast::<R>().ok())
            .map(|boxed| *boxed)
    }

    pub fn contains<R: Resource>(&self) -> bool {
        let type_id = TypeId::of::<R>();
        self.data.contains_key(&type_id)
    }
}

pub type Entity = u64;

#[derive(Clone, Debug)]
pub struct Name(pub String);

#[derive(Clone, Debug)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    pub fn to_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::from_translation(self.translation);
        let rotation = Matrix4::from(self.rotation);
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        translation * rotation * scale
    }
}

#[derive(Clone, Debug)]
pub struct GlobalTransform(pub Matrix4<f32>);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Matrix4::identity())
    }
}

impl GlobalTransform {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Visible(pub bool);

#[derive(Clone, Debug)]
pub struct Parent(pub Entity);

#[derive(Clone, Debug, Default)]
pub struct Children(pub Vec<Entity>);

#[derive(Clone, Debug)]
pub struct MeshRef {
    pub mesh_asset_id: AssetId,
    pub object_index: usize,
}

#[derive(Clone, Debug)]
pub struct MaterialRef(pub AssetId);

#[derive(Clone, Debug)]
pub struct SkeletonRef(pub AssetId);

#[derive(Clone, Debug, Default)]
pub struct AnimationState {
    pub current_clip_id: Option<AnimationClipId>,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
    pub looping: bool,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            current_clip_id: None,
            time: 0.0,
            speed: 1.0,
            playing: true,
            looping: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LineRendering {
    pub line_width: f32,
}

impl LineRendering {
    pub fn new(line_width: f32) -> Self {
        Self { line_width }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BillboardBehavior {
    pub always_face_camera: bool,
}

impl BillboardBehavior {
    pub fn new(always_face_camera: bool) -> Self {
        Self { always_face_camera }
    }
}

#[derive(Clone, Debug)]
pub struct NodeRef(pub AssetId);

#[derive(Clone, Debug)]
pub struct SkinRef {
    pub skeleton_asset_id: AssetId,
    pub joint_indices: Vec<u32>,
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
}

pub struct World {
    next_entity: Entity,
    resources: Resources,
    components: crate::ecs::storage::Components,
    component_registry: crate::ecs::registry::ComponentRegistry,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("entity_count", &self.entity_count())
            .field("resources", &self.resources)
            .field("components", &self.components)
            .finish()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            next_entity: 1,
            resources: Resources::new(),
            components: crate::ecs::storage::Components::new(),
            component_registry: crate::ecs::registry::ComponentRegistry::new(),
        };

        world.register_component::<Name>();
        world.register_component::<Transform>();
        world.register_component::<GlobalTransform>();
        world.register_component::<Visible>();
        world.register_component::<Parent>();
        world.register_component::<Children>();
        world.register_component::<MeshRef>();
        world.register_component::<MaterialRef>();
        world.register_component::<SkeletonRef>();
        world.register_component::<AnimationState>();
        world.register_component::<LineRendering>();
        world.register_component::<BillboardBehavior>();
        world.register_component::<NodeRef>();
        world.register_component::<SkinRef>();
        world.register_component::<Animated>();
        world.register_component::<Skinned>();
        world.register_component::<Model>();
        world.register_component::<MeshHandle>();

        world
    }

    pub fn register_component<T: crate::ecs::storage::Component>(&mut self) {
        self.component_registry.register::<T>();
        self.components.register::<T>();
    }

    pub fn get_component<T: crate::ecs::storage::Component>(&self, entity: Entity) -> Option<&T> {
        self.components.get::<T>(entity)
    }

    pub fn get_component_mut<T: crate::ecs::storage::Component>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        self.components.get_mut::<T>(entity)
    }

    pub fn get_component_ref<T: crate::ecs::storage::Component>(
        &self,
        entity: Entity,
    ) -> Option<&T> {
        self.components.get::<T>(entity)
    }

    pub fn get_component_ref_mut<T: crate::ecs::storage::Component>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        self.components.get_mut::<T>(entity)
    }

    pub fn insert_component<T: crate::ecs::storage::Component>(
        &mut self,
        entity: Entity,
        component: T,
    ) {
        self.components.insert(entity, component);
    }

    pub fn has_component<T: crate::ecs::storage::Component>(&self, entity: Entity) -> bool {
        self.components.contains::<T>(entity)
    }

    pub fn remove_component<T: crate::ecs::storage::Component>(&mut self, entity: Entity) {
        self.components.remove::<T>(entity);
    }

    pub fn component_entities<T: crate::ecs::storage::Component>(&self) -> Vec<Entity> {
        self.components.entities::<T>()
    }

    pub fn components(&self) -> &crate::ecs::storage::Components {
        &self.components
    }

    pub fn components_mut(&mut self) -> &mut crate::ecs::storage::Components {
        &mut self.components
    }

    pub fn component_registry(&self) -> &crate::ecs::registry::ComponentRegistry {
        &self.component_registry
    }

    pub fn storage<T: crate::ecs::storage::Component>(
        &self,
    ) -> Option<&crate::ecs::storage::TypedStorage<T>> {
        self.components.storage::<T>()
    }

    pub fn storage_mut<T: crate::ecs::storage::Component>(
        &mut self,
    ) -> Option<&mut crate::ecs::storage::TypedStorage<T>> {
        self.components.storage_mut::<T>()
    }

    pub fn iter_components<T: crate::ecs::storage::Component>(
        &self,
    ) -> impl Iterator<Item = (Entity, &T)> {
        self.components
            .storage::<T>()
            .into_iter()
            .flat_map(|s| s.iter())
    }

    pub fn iter_components_mut<T: crate::ecs::storage::Component>(
        &mut self,
    ) -> impl Iterator<Item = (Entity, &mut T)> {
        self.components
            .storage_mut::<T>()
            .into_iter()
            .flat_map(|s| s.iter_mut())
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(resource);
    }

    pub fn resource<R: Resource>(&self) -> ResRef<R> {
        self.resources.get::<R>().expect(&format!(
            "Resource {} not found",
            std::any::type_name::<R>()
        ))
    }

    pub fn resource_mut<R: Resource>(&self) -> ResMut<R> {
        self.resources.get_mut::<R>().expect(&format!(
            "Resource {} not found",
            std::any::type_name::<R>()
        ))
    }

    pub fn get_resource<R: Resource>(&self) -> Option<ResRef<R>> {
        self.resources.get::<R>()
    }

    pub fn get_resource_mut<R: Resource>(&self) -> Option<ResMut<R>> {
        self.resources.get_mut::<R>()
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.resources.remove::<R>()
    }

    pub fn contains_resource<R: Resource>(&self) -> bool {
        self.resources.contains::<R>()
    }

    pub fn spawn(&mut self) -> Entity {
        let entity = self.next_entity;
        self.next_entity += 1;
        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.components.remove_entity(entity);
    }

    pub fn query_renderable(&self) -> Vec<Entity> {
        self.iter_components::<MeshRef>()
            .filter(|(e, _)| {
                self.get_component::<Visible>(*e)
                    .map(|v| v.0)
                    .unwrap_or(true)
            })
            .map(|(e, _)| e)
            .collect()
    }

    pub fn query_animated(&self) -> Vec<Entity> {
        self.iter_components::<AnimationState>()
            .map(|(e, _)| e)
            .collect()
    }

    pub fn query_skinned(&self) -> Vec<Entity> {
        self.iter_components::<SkinRef>().map(|(e, _)| e).collect()
    }

    pub fn query_line_rendering(&self) -> Vec<Entity> {
        self.iter_components::<LineRendering>()
            .map(|(e, _)| e)
            .collect()
    }

    pub fn query_billboards(&self) -> Vec<Entity> {
        self.iter_components::<BillboardBehavior>()
            .map(|(e, _)| e)
            .collect()
    }

    pub fn query_with_parent(&self) -> Vec<Entity> {
        self.iter_components::<Parent>().map(|(e, _)| e).collect()
    }

    pub fn get_root_entities(&self) -> Vec<Entity> {
        self.iter_components::<Transform>()
            .filter(|(e, _)| !self.has_component::<Parent>(*e))
            .map(|(e, _)| e)
            .collect()
    }

    pub fn iter_models(&self) -> impl Iterator<Item = (Entity, &MeshHandle)> {
        self.iter_components::<MeshHandle>()
            .filter(|(e, _)| self.has_component::<Model>(*e))
    }

    pub fn iter_animated_entities(&self) -> impl Iterator<Item = (Entity, &AnimationState)> {
        self.iter_components::<AnimationState>()
            .filter(|(e, _)| self.has_component::<Animated>(*e))
    }

    pub fn iter_skinned_entities(&self) -> impl Iterator<Item = (Entity, &SkinRef)> {
        self.iter_components::<SkinRef>()
            .filter(|(e, _)| self.has_component::<Skinned>(*e))
    }

    pub fn entity_count(&self) -> usize {
        self.component_entities::<Transform>().len()
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    pub fn add_child(&mut self, parent: Entity, child: Entity) {
        if let Some(children) = self.get_component_mut::<Children>(parent) {
            children.0.push(child);
        } else {
            self.insert_component(parent, Children(vec![child]));
        }
    }
}

pub struct EntityBuilder<'a> {
    world: &'a mut World,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    pub fn new(world: &'a mut World) -> Self {
        let entity = world.spawn();
        Self { world, entity }
    }

    pub fn with_name(self, name: &str) -> Self {
        self.world
            .insert_component(self.entity, Name(name.to_string()));
        self
    }

    pub fn with_transform(self, transform: Transform) -> Self {
        self.world.insert_component(self.entity, transform);
        self.world
            .insert_component(self.entity, GlobalTransform::new());
        self
    }

    pub fn with_visible(self, visible: bool) -> Self {
        self.world.insert_component(self.entity, Visible(visible));
        self
    }

    pub fn with_parent(self, parent: Entity) -> Self {
        self.world.insert_component(self.entity, Parent(parent));
        self.world.add_child(parent, self.entity);
        self
    }

    pub fn with_mesh(self, mesh_asset_id: AssetId, object_index: usize) -> Self {
        self.world.insert_component(
            self.entity,
            MeshRef {
                mesh_asset_id,
                object_index,
            },
        );
        self
    }

    pub fn with_material(self, material_asset_id: AssetId) -> Self {
        self.world
            .insert_component(self.entity, MaterialRef(material_asset_id));
        self
    }

    pub fn with_skeleton(self, skeleton_asset_id: AssetId) -> Self {
        self.world
            .insert_component(self.entity, SkeletonRef(skeleton_asset_id));
        self
    }

    pub fn with_animation_state(self, state: AnimationState) -> Self {
        self.world.insert_component(self.entity, state);
        self
    }

    pub fn with_line_rendering(self, line_width: f32) -> Self {
        self.world
            .insert_component(self.entity, LineRendering::new(line_width));
        self
    }

    pub fn with_billboard(self, always_face_camera: bool) -> Self {
        self.world
            .insert_component(self.entity, BillboardBehavior::new(always_face_camera));
        self
    }

    pub fn with_node(self, node_asset_id: AssetId) -> Self {
        self.world
            .insert_component(self.entity, NodeRef(node_asset_id));
        self
    }

    pub fn with_skin(
        self,
        skeleton_asset_id: AssetId,
        joint_indices: Vec<u32>,
        inverse_bind_matrices: Vec<Matrix4<f32>>,
    ) -> Self {
        self.world.insert_component(
            self.entity,
            SkinRef {
                skeleton_asset_id,
                joint_indices,
                inverse_bind_matrices,
            },
        );
        self
    }

    pub fn build(self) -> Entity {
        self.entity
    }
}

impl World {
    pub fn entity(&mut self) -> EntityBuilder {
        EntityBuilder::new(self)
    }

    pub fn query<Q: crate::ecs::query::QueryData>(&self) -> crate::ecs::query::Query<Q> {
        crate::ecs::query::Query::new(self)
    }

    pub fn query_filtered<Q: crate::ecs::query::QueryData, F: crate::ecs::query::QueryFilter>(
        &self,
    ) -> crate::ecs::query::QueryFiltered<Q, F> {
        crate::ecs::query::QueryFiltered::new(self)
    }

    pub fn query_builder(&self) -> crate::ecs::query::QueryBuilder<crate::ecs::query::HNil> {
        crate::ecs::query::QueryBuilder::new(self)
    }
}
