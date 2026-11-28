use glam::Mat4;

use crate::{GpuMeshRef, MeshId, RenderInfo};

#[derive(Clone, Copy, Debug, Default)]
pub struct ObjectIndex(pub usize);

impl ObjectIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MeshHandle {
    pub mesh_id: MeshId,
}

impl MeshHandle {
    pub fn new(mesh_id: MeshId) -> Self {
        Self { mesh_id }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkeletonHandle {
    pub skeleton_id: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderData {
    pub mesh_ref: GpuMeshRef,
    pub render_info: RenderInfo,
    pub model_matrix: Mat4,
}

impl Default for RenderData {
    fn default() -> Self {
        Self {
            mesh_ref: GpuMeshRef::default(),
            render_info: RenderInfo::default(),
            model_matrix: Mat4::IDENTITY,
        }
    }
}

impl RenderData {
    pub fn new(mesh_ref: GpuMeshRef, render_info: RenderInfo) -> Self {
        Self {
            mesh_ref,
            render_info,
            model_matrix: Mat4::IDENTITY,
        }
    }

    pub fn with_model_matrix(mut self, model_matrix: Mat4) -> Self {
        self.model_matrix = model_matrix;
        self
    }
}
