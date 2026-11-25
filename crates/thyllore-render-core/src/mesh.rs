use crate::{IndexBufferHandle, PipelineId, VertexBufferHandle};

pub const FRAMES_IN_FLIGHT: usize = 2;

pub trait MeshBuffers {
    fn vertex_buffer_handle(&self, frame_slot: usize) -> VertexBufferHandle;
    fn index_buffer_handle(&self, frame_slot: usize) -> IndexBufferHandle;
    fn index_count(&self) -> u32;
}

#[derive(Clone, Debug)]
pub struct StaticMesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
}

impl<V> Default for StaticMesh<V> {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer_handle: VertexBufferHandle::INVALID,
            index_buffer_handle: IndexBufferHandle::INVALID,
        }
    }
}

impl<V> MeshBuffers for StaticMesh<V> {
    fn vertex_buffer_handle(&self, _frame_slot: usize) -> VertexBufferHandle {
        self.vertex_buffer_handle
    }

    fn index_buffer_handle(&self, _frame_slot: usize) -> IndexBufferHandle {
        self.index_buffer_handle
    }

    fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }
}

#[derive(Clone, Debug)]
pub struct DynamicMesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handles: [VertexBufferHandle; FRAMES_IN_FLIGHT],
    pub index_buffer_handles: [IndexBufferHandle; FRAMES_IN_FLIGHT],
}

impl<V> Default for DynamicMesh<V> {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer_handles: [VertexBufferHandle::INVALID; FRAMES_IN_FLIGHT],
            index_buffer_handles: [IndexBufferHandle::INVALID; FRAMES_IN_FLIGHT],
        }
    }
}

impl<V> DynamicMesh<V> {
    pub fn new() -> Self
    where
        V: Default,
    {
        Self::default()
    }
}

impl<V> MeshBuffers for DynamicMesh<V> {
    fn vertex_buffer_handle(&self, frame_slot: usize) -> VertexBufferHandle {
        self.vertex_buffer_handles[frame_slot]
    }

    fn index_buffer_handle(&self, frame_slot: usize) -> IndexBufferHandle {
        self.index_buffer_handles[frame_slot]
    }

    fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GpuMeshRef {
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
    pub index_count: u32,
}

impl GpuMeshRef {
    pub fn new(
        vertex_buffer_handle: VertexBufferHandle,
        index_buffer_handle: IndexBufferHandle,
        index_count: u32,
    ) -> Self {
        Self {
            vertex_buffer_handle,
            index_buffer_handle,
            index_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderInfo {
    pub pipeline_id: Option<PipelineId>,
    pub object_index: usize,
}

impl RenderInfo {
    pub fn new(pipeline_id: Option<PipelineId>, object_index: usize) -> Self {
        Self {
            pipeline_id,
            object_index,
        }
    }
}

pub type LineMesh = DynamicMesh<crate::ColorVertex>;
pub type StaticLineMesh = StaticMesh<crate::ColorVertex>;

#[derive(Clone, Copy, Debug, Default)]
pub struct MeshScale(pub f32);

impl MeshScale {
    pub fn new(scale: f32) -> Self {
        Self(scale)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}
