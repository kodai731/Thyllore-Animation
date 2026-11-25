pub(crate) mod components;
pub(crate) mod systems;

pub use components::texture_remap::MaterialTextureRemap;
pub use systems::texture_remap_io::{
    load_material_texture_remap, make_model_relative_path, material_texture_remap_path,
    resolve_remapped_texture, save_material_texture_remap,
};
