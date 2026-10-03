mod gpu;
pub mod presets;

pub use gpu::{DynamicMesh, GpuMeshRef, LineMesh, MeshScale, RenderInfo};
pub use thyllore_model_core::mesh::{
    MeshData, PrimitiveTopology, VertexAttribute, VertexAttributeId, VertexAttributeValues,
    VertexFormat, VertexLayout,
};
