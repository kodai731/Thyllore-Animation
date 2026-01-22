use std::collections::HashMap;

use cgmath::{Matrix4, SquareMatrix};

use crate::animation::{AnimationClip, AnimationClipId, Skeleton, SkeletonId};
use crate::render::MaterialUBO;
use crate::app::graphics_resource::MaterialId;

pub type AssetId = u64;

#[derive(Clone, Debug, Default)]
pub struct MeshAsset {
    pub id: AssetId,
    pub name: String,
    pub graphics_mesh_index: usize,
    pub object_index: usize,
    pub material_id: Option<MaterialId>,
    pub skeleton_id: Option<SkeletonId>,
    pub node_index: Option<usize>,
    pub render_to_gbuffer: bool,
}

#[derive(Clone, Debug)]
pub struct MaterialAsset {
    pub id: AssetId,
    pub name: String,
    pub material_id: MaterialId,
    pub properties: MaterialUBO,
}

#[derive(Clone, Debug)]
pub struct SkeletonAsset {
    pub id: AssetId,
    pub skeleton_id: SkeletonId,
    pub skeleton: Skeleton,
}

#[derive(Clone, Debug)]
pub struct AnimationClipAsset {
    pub id: AssetId,
    pub clip_id: AnimationClipId,
    pub clip: AnimationClip,
}

#[derive(Clone, Debug)]
pub struct NodeAsset {
    pub id: AssetId,
    pub name: String,
    pub parent_id: Option<AssetId>,
    pub local_transform: Matrix4<f32>,
}

impl Default for NodeAsset {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            parent_id: None,
            local_transform: Matrix4::identity(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AssetStorage {
    pub meshes: HashMap<AssetId, MeshAsset>,
    pub materials: HashMap<AssetId, MaterialAsset>,
    pub skeletons: HashMap<AssetId, SkeletonAsset>,
    pub animation_clips: HashMap<AssetId, AnimationClipAsset>,
    pub nodes: HashMap<AssetId, NodeAsset>,
    next_id: AssetId,
}

impl AssetStorage {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            materials: HashMap::new(),
            skeletons: HashMap::new(),
            animation_clips: HashMap::new(),
            nodes: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn allocate_id(&mut self) -> AssetId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_mesh(&mut self, mut mesh: MeshAsset) -> AssetId {
        let id = self.allocate_id();
        mesh.id = id;
        self.meshes.insert(id, mesh);
        id
    }

    pub fn add_material(&mut self, mut material: MaterialAsset) -> AssetId {
        let id = self.allocate_id();
        material.id = id;
        self.materials.insert(id, material);
        id
    }

    pub fn add_skeleton(&mut self, mut skeleton: SkeletonAsset) -> AssetId {
        let id = self.allocate_id();
        skeleton.id = id;
        self.skeletons.insert(id, skeleton);
        id
    }

    pub fn add_animation_clip(&mut self, mut clip: AnimationClipAsset) -> AssetId {
        let id = self.allocate_id();
        clip.id = id;
        self.animation_clips.insert(id, clip);
        id
    }

    pub fn add_node(&mut self, mut node: NodeAsset) -> AssetId {
        let id = self.allocate_id();
        node.id = id;
        self.nodes.insert(id, node);
        id
    }

    pub fn get_mesh(&self, id: AssetId) -> Option<&MeshAsset> {
        self.meshes.get(&id)
    }

    pub fn get_mesh_mut(&mut self, id: AssetId) -> Option<&mut MeshAsset> {
        self.meshes.get_mut(&id)
    }

    pub fn get_material(&self, id: AssetId) -> Option<&MaterialAsset> {
        self.materials.get(&id)
    }

    pub fn get_skeleton(&self, id: AssetId) -> Option<&SkeletonAsset> {
        self.skeletons.get(&id)
    }

    pub fn get_skeleton_mut(&mut self, id: AssetId) -> Option<&mut SkeletonAsset> {
        self.skeletons.get_mut(&id)
    }

    pub fn get_animation_clip(&self, id: AssetId) -> Option<&AnimationClipAsset> {
        self.animation_clips.get(&id)
    }

    pub fn get_node(&self, id: AssetId) -> Option<&NodeAsset> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: AssetId) -> Option<&mut NodeAsset> {
        self.nodes.get_mut(&id)
    }

    pub fn clear(&mut self) {
        self.meshes.clear();
        self.materials.clear();
        self.skeletons.clear();
        self.animation_clips.clear();
        self.nodes.clear();
    }
}
