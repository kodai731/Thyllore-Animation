mod attribute;
mod gpu_mesh;
mod interleave;
mod mesh_data;
pub mod presets;
mod values;

pub use crate::ecs::systems::mesh_systems::create_interleaved_buffer;
pub use attribute::{VertexAttribute, VertexAttributeId, VertexFormat};
pub use gpu_mesh::{DynamicMesh, GpuMeshRef, LineMesh, MeshScale, RenderInfo};
pub use interleave::VertexLayout;
pub use mesh_data::{MeshData, PrimitiveTopology};
pub use values::VertexAttributeValues;
