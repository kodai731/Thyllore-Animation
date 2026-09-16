use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Result;

use crate::loader::{load_png_image, LoadedMesh, ModelLoadResult, TextureData, TextureSource};
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::{MeshSource, TexturePixels};
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::vulkan::Instance;

pub(super) unsafe fn upload_model_meshes(
    load_result: &ModelLoadResult,
    model_path: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
) -> Result<()> {
    graphics.ensure_object_capacity(
        instance,
        device,
        swapchain.swapchain_images.len(),
        load_result.meshes.len(),
    )?;

    for loaded_mesh in &load_result.meshes {
        let texture = resolve_texture_pixels(loaded_mesh, model_path);
        let source = MeshSource {
            vertex_data: &loaded_mesh.vertex_data,
            base_vertices: &loaded_mesh.local_vertices,
            skin_data: loaded_mesh.skin_data.as_ref(),
            skeleton_id: loaded_mesh.skeleton_id,
            node_index: loaded_mesh.node_index,
            texture: TexturePixels {
                rgba: &texture.data,
                width: texture.width,
                height: texture.height,
            },
            base_color_factor: loaded_mesh.base_color_factor,
        };
        graphics.push_mesh(instance, device, command_pool, &source)?;
    }

    Ok(())
}

fn resolve_texture_pixels(loaded_mesh: &LoadedMesh, model_path: &str) -> TextureData {
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
