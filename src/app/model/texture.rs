use std::path::{Path, PathBuf};

use crate::loader::{load_png_image, LoadedMesh, TextureData, TextureSource};

pub(super) fn resolve_texture_pixels(loaded_mesh: &LoadedMesh, model_path: &str) -> TextureData {
    match &loaded_mesh.texture {
        Some(TextureSource::Embedded(texture)) => texture.clone(),
        Some(TextureSource::File(texture_path)) => {
            let resolved = resolve_texture_path(texture_path, model_path);
            let load_path = resolved.to_string_lossy();
            match load_png_image(&load_path) {
                Ok((data, width, height)) => TextureData {
                    data,
                    width,
                    height,
                },
                Err(e) => {
                    log_warn!("Failed to load texture {}: {}", load_path, e);
                    white_texture()
                }
            }
        }
        None => white_texture(),
    }
}

fn white_texture() -> TextureData {
    TextureData {
        data: vec![255u8, 255, 255, 255],
        width: 1,
        height: 1,
    }
}

fn resolve_texture_path(texture_path: &str, model_path: &str) -> PathBuf {
    let original = Path::new(texture_path);
    if original.exists() {
        return original.to_path_buf();
    }

    let file_stem = original.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_name = original.file_name().and_then(|s| s.to_str()).unwrap_or("");

    let model_dir = Path::new(model_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let model_root = model_dir.parent().unwrap_or(model_dir);

    let texture_dir = original.parent().unwrap_or_else(|| Path::new("."));
    let texture_root = texture_dir.parent().unwrap_or(texture_dir);

    let mut search_dirs = vec![
        model_dir.to_path_buf(),
        model_dir.join("textures"),
        model_root.join("textures"),
    ];

    if texture_dir != model_dir {
        search_dirs.push(texture_dir.to_path_buf());
        search_dirs.push(texture_dir.join("textures"));
        search_dirs.push(texture_root.join("textures"));
    }

    let candidate_names: Vec<String> = vec![
        file_name.to_string(),
        format!("{}.png", file_name),
        format!("{}.png", file_stem),
        format!("{}.jpg", file_stem),
    ];

    for dir in &search_dirs {
        for name in &candidate_names {
            let candidate = dir.join(name);
            if candidate.exists() {
                log!(
                    "Resolved texture: {} -> {}",
                    texture_path,
                    candidate.display()
                );
                return candidate;
            }
        }
    }

    log!("Texture not found, using original path: {}", texture_path);
    original.to_path_buf()
}
