#[macro_use]
extern crate thyllore_log_core;

pub mod fbx;
pub mod gltf;
mod model_result;
pub mod primitive;
mod texture;

pub use model_result::{LoadedMesh, LoadedNode, ModelLoadResult, TextureData, TextureSource};
pub use texture::{find_texture_file, load_png_image, resolve_mesh_texture, DecodedTextureFiles};
