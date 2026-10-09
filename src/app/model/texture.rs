use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thyllore_avatar_core::material::components::texture_remap::MaterialTextureRemap;
use thyllore_avatar_core::material::systems::texture_remap_io::resolve_remapped_texture;

use crate::loader::{find_texture_file, load_png_image, LoadedMesh, TextureData, TextureSource};

pub(super) type DecodedTextureFiles = HashMap<PathBuf, TextureData>;

pub(super) fn resolve_texture_pixels(
    loaded_mesh: &LoadedMesh,
    model_path: &str,
    remap: &MaterialTextureRemap,
    decoded_files: &mut DecodedTextureFiles,
) -> TextureData {
    let remapped_file =
        resolve_remapped_texture(remap, &loaded_mesh.material_name, Path::new(model_path));
    if let Some(texture_file) = remapped_file {
        return decode_texture_file(texture_file, decoded_files);
    }

    match &loaded_mesh.texture {
        Some(TextureSource::Embedded(texture)) => texture.clone(),
        Some(TextureSource::File(reference)) => {
            match find_texture_file(reference, Path::new(model_path)) {
                Some(texture_file) => decode_texture_file(texture_file, decoded_files),
                None => {
                    log_warn!(
                        "Texture {} of material {} not found",
                        reference,
                        loaded_mesh.material_name
                    );
                    white_texture()
                }
            }
        }
        None => white_texture(),
    }
}

fn decode_texture_file(
    texture_file: PathBuf,
    decoded_files: &mut DecodedTextureFiles,
) -> TextureData {
    if let Some(texture) = decoded_files.get(&texture_file) {
        return texture.clone();
    }

    let texture = match load_png_image(&texture_file.to_string_lossy()) {
        Ok((data, width, height)) => {
            log!("Loaded texture {}", texture_file.display());
            TextureData {
                data,
                width,
                height,
            }
        }
        Err(error) => {
            log_warn!(
                "Failed to load texture {}: {}",
                texture_file.display(),
                error
            );
            white_texture()
        }
    };

    decoded_files.insert(texture_file, texture.clone());
    texture
}

fn white_texture() -> TextureData {
    TextureData {
        data: vec![255u8, 255, 255, 255],
        width: 1,
        height: 1,
    }
}
