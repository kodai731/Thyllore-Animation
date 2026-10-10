use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{find_texture_file, load_png_image};
use crate::model_result::{LoadedMesh, TextureData, TextureSource};

pub type DecodedTextureFiles = HashMap<PathBuf, TextureData>;

/// The pixels a mesh is drawn with: a remapped file wins over the model's own texture, and a missing
/// texture falls back to white. `decoded_files` keeps each file decoded once per model.
pub fn resolve_mesh_texture(
    loaded_mesh: &LoadedMesh,
    model_path: &Path,
    remapped_file: Option<PathBuf>,
    decoded_files: &mut DecodedTextureFiles,
) -> TextureData {
    if let Some(texture_file) = remapped_file {
        return decode_texture_file(texture_file, decoded_files);
    }

    match &loaded_mesh.texture {
        Some(TextureSource::Embedded(texture)) => texture.clone(),
        Some(TextureSource::File(reference)) => match find_texture_file(reference, model_path) {
            Some(texture_file) => decode_texture_file(texture_file, decoded_files),
            None => {
                log_warn!(
                    "Texture {} of material {} not found",
                    reference,
                    loaded_mesh.material_name
                );
                white_texture()
            }
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(path: &Path, rgba: [u8; 4]) {
        let file = std::fs::File::create(path).expect("create png");
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(&rgba).expect("png data");
    }

    fn mesh_with_texture(texture: Option<TextureSource>) -> LoadedMesh {
        LoadedMesh {
            texture,
            ..LoadedMesh::default()
        }
    }

    #[test]
    fn embedded_texture_is_used_as_is() {
        let embedded = TextureData {
            data: vec![1, 2, 3, 4],
            width: 1,
            height: 1,
        };
        let mesh = mesh_with_texture(Some(TextureSource::Embedded(embedded)));

        let texture = resolve_mesh_texture(
            &mesh,
            Path::new("model.fbx"),
            None,
            &mut DecodedTextureFiles::new(),
        );

        assert_eq!(texture.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn missing_texture_falls_back_to_white() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model_path = dir.path().join("model.fbx");
        let mut decoded = DecodedTextureFiles::new();

        let untextured =
            resolve_mesh_texture(&mesh_with_texture(None), &model_path, None, &mut decoded);
        let unresolved = resolve_mesh_texture(
            &mesh_with_texture(Some(TextureSource::File("absent.png".to_string()))),
            &model_path,
            None,
            &mut decoded,
        );

        assert_eq!(untextured.data, white_texture().data);
        assert_eq!(unresolved.data, white_texture().data);
    }

    #[test]
    fn remapped_file_wins_over_the_model_texture_and_is_decoded_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let remapped = dir.path().join("remapped.png");
        write_png(&remapped, [10, 20, 30, 255]);
        let mesh = mesh_with_texture(Some(TextureSource::File("absent.png".to_string())));
        let mut decoded = DecodedTextureFiles::new();

        let texture = resolve_mesh_texture(
            &mesh,
            &dir.path().join("model.fbx"),
            Some(remapped.clone()),
            &mut decoded,
        );

        assert_eq!(texture.data, vec![10, 20, 30, 255]);
        assert_eq!(decoded.len(), 1);
        assert!(decoded.contains_key(&remapped));
    }
}
