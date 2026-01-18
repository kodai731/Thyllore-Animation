use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::animation::AnimationClipId;
use crate::asset::AssetId;

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

#[derive(Default)]
pub struct World {
    next_entity: Entity,
    resources: Resources,

    pub names: HashMap<Entity, Name>,
    pub transforms: HashMap<Entity, Transform>,
    pub global_transforms: HashMap<Entity, GlobalTransform>,
    pub visibles: HashMap<Entity, Visible>,
    pub parents: HashMap<Entity, Parent>,
    pub children: HashMap<Entity, Children>,
    pub mesh_refs: HashMap<Entity, MeshRef>,
    pub material_refs: HashMap<Entity, MaterialRef>,
    pub skeleton_refs: HashMap<Entity, SkeletonRef>,
    pub animation_states: HashMap<Entity, AnimationState>,
    pub line_renderings: HashMap<Entity, LineRendering>,
    pub billboard_behaviors: HashMap<Entity, BillboardBehavior>,
    pub node_refs: HashMap<Entity, NodeRef>,
    pub skin_refs: HashMap<Entity, SkinRef>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("entity_count", &self.transforms.len())
            .field("resources", &self.resources)
            .finish()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity: 1,
            resources: Resources::new(),
            names: HashMap::new(),
            transforms: HashMap::new(),
            global_transforms: HashMap::new(),
            visibles: HashMap::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
            mesh_refs: HashMap::new(),
            material_refs: HashMap::new(),
            skeleton_refs: HashMap::new(),
            animation_states: HashMap::new(),
            line_renderings: HashMap::new(),
            billboard_behaviors: HashMap::new(),
            node_refs: HashMap::new(),
            skin_refs: HashMap::new(),
        }
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
        self.names.remove(&entity);
        self.transforms.remove(&entity);
        self.global_transforms.remove(&entity);
        self.visibles.remove(&entity);
        self.parents.remove(&entity);
        self.children.remove(&entity);
        self.mesh_refs.remove(&entity);
        self.material_refs.remove(&entity);
        self.skeleton_refs.remove(&entity);
        self.animation_states.remove(&entity);
        self.line_renderings.remove(&entity);
        self.billboard_behaviors.remove(&entity);
        self.node_refs.remove(&entity);
        self.skin_refs.remove(&entity);
    }

    pub fn query_renderable(&self) -> Vec<Entity> {
        self.mesh_refs
            .keys()
            .filter(|e| self.visibles.get(e).map(|v| v.0).unwrap_or(true))
            .copied()
            .collect()
    }

    pub fn query_animated(&self) -> Vec<Entity> {
        self.animation_states.keys().copied().collect()
    }

    pub fn query_skinned(&self) -> Vec<Entity> {
        self.skin_refs.keys().copied().collect()
    }

    pub fn query_line_rendering(&self) -> Vec<Entity> {
        self.line_renderings.keys().copied().collect()
    }

    pub fn query_billboards(&self) -> Vec<Entity> {
        self.billboard_behaviors.keys().copied().collect()
    }

    pub fn query_with_parent(&self) -> Vec<Entity> {
        self.parents.keys().copied().collect()
    }

    pub fn get_root_entities(&self) -> Vec<Entity> {
        self.transforms
            .keys()
            .filter(|e| !self.parents.contains_key(e))
            .copied()
            .collect()
    }

    pub fn entity_count(&self) -> usize {
        self.transforms.len()
    }

    pub fn clear(&mut self) {
        self.names.clear();
        self.transforms.clear();
        self.global_transforms.clear();
        self.visibles.clear();
        self.parents.clear();
        self.children.clear();
        self.mesh_refs.clear();
        self.material_refs.clear();
        self.skeleton_refs.clear();
        self.animation_states.clear();
        self.line_renderings.clear();
        self.billboard_behaviors.clear();
        self.node_refs.clear();
        self.skin_refs.clear();
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
        self.world.names.insert(self.entity, Name(name.to_string()));
        self
    }

    pub fn with_transform(self, transform: Transform) -> Self {
        self.world.transforms.insert(self.entity, transform);
        self.world
            .global_transforms
            .insert(self.entity, GlobalTransform::new());
        self
    }

    pub fn with_visible(self, visible: bool) -> Self {
        self.world.visibles.insert(self.entity, Visible(visible));
        self
    }

    pub fn with_parent(self, parent: Entity) -> Self {
        self.world.parents.insert(self.entity, Parent(parent));

        let children = self
            .world
            .children
            .entry(parent)
            .or_insert_with(|| Children(Vec::new()));
        children.0.push(self.entity);

        self
    }

    pub fn with_mesh(self, mesh_asset_id: AssetId, object_index: usize) -> Self {
        self.world.mesh_refs.insert(
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
            .material_refs
            .insert(self.entity, MaterialRef(material_asset_id));
        self
    }

    pub fn with_skeleton(self, skeleton_asset_id: AssetId) -> Self {
        self.world
            .skeleton_refs
            .insert(self.entity, SkeletonRef(skeleton_asset_id));
        self
    }

    pub fn with_animation_state(self, state: AnimationState) -> Self {
        self.world.animation_states.insert(self.entity, state);
        self
    }

    pub fn with_line_rendering(self, line_width: f32) -> Self {
        self.world
            .line_renderings
            .insert(self.entity, LineRendering::new(line_width));
        self
    }

    pub fn with_billboard(self, always_face_camera: bool) -> Self {
        self.world
            .billboard_behaviors
            .insert(self.entity, BillboardBehavior::new(always_face_camera));
        self
    }

    pub fn with_node(self, node_asset_id: AssetId) -> Self {
        self.world
            .node_refs
            .insert(self.entity, NodeRef(node_asset_id));
        self
    }

    pub fn with_skin(
        self,
        skeleton_asset_id: AssetId,
        joint_indices: Vec<u32>,
        inverse_bind_matrices: Vec<Matrix4<f32>>,
    ) -> Self {
        self.world.skin_refs.insert(
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
}
