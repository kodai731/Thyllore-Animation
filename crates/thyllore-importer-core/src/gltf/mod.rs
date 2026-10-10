mod loader;
mod morph;
pub mod spring_bone_extension;
pub mod vrm_humanoid_extension;

#[cfg(feature = "auto-rig")]
pub use loader::load_gltf_from_slice;
pub use loader::{load_gltf_file, GltfLoadResult, GltfMeshData, ImageData, NodeInfo};
