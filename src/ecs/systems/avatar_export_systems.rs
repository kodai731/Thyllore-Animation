use std::fs;
use std::path::{Path, PathBuf};

use thyllore_exporter_core::systems::unity::anim::write_unity_anim;
use thyllore_exporter_core::systems::unity::curves::{morph_track_curves, write_expression_anim};

use crate::asset::AssetStorage;
use crate::ecs::resource::{ClipLibrary, ExpressionLibraryState, TimelineState};
use crate::ecs::systems::avatar_setup_systems::{find_expression_morph, find_model_path};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

fn unity_export_dir(model_path: &Path) -> PathBuf {
    let stem = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    model_path.with_file_name(format!("{}_unity", stem))
}

fn create_unity_export_dir(model_path: &str) -> Option<PathBuf> {
    let export_dir = unity_export_dir(Path::new(model_path));
    match fs::create_dir_all(&export_dir) {
        Ok(()) => Some(export_dir),
        Err(error) => {
            log_warn!(
                "Failed to create Unity export directory {}: {}",
                export_dir.display(),
                error
            );
            None
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn export_expression_anims(world: &World, assets: &AssetStorage, graphics: &GraphicsResources) {
    let Some(model_path) = find_model_path(world) else {
        msg_error!("Cannot export expression anims: no model loaded");
        return;
    };

    let library_state = world.resource::<ExpressionLibraryState>();
    let presets = &library_state.library.presets;

    let morph = match find_expression_morph(world, assets, graphics) {
        Some(m) => m,
        None => {
            msg_error!("Cannot export expression anims: no expression mesh found");
            return;
        }
    };

    let source_mesh = &morph.source_mesh;
    if source_mesh.is_empty() {
        msg_error!("Cannot export expression anims: expression mesh has no source_mesh path");
        return;
    }

    let Some(export_dir) = create_unity_export_dir(&model_path) else {
        return;
    };
    let mut written = 0usize;

    for preset in presets {
        if preset.weights.is_empty() {
            continue;
        }
        let filename = format!("{}.anim", sanitize_filename(&preset.name));
        let path = export_dir.join(&filename);
        let yaml = write_expression_anim(&preset.name, source_mesh, &preset.weights);
        match fs::write(&path, yaml) {
            Ok(()) => written += 1,
            Err(error) => msg_error!("Failed to write {}: {}", path.display(), error),
        }
    }

    msg_info!(
        "Wrote {} expression .anim file(s) to {}",
        written,
        export_dir.display()
    );
}

pub fn export_morph_track_anim(world: &World, _assets: &AssetStorage) {
    let timeline = world.resource::<TimelineState>();
    let clip_id = match timeline.current_clip_id {
        Some(id) => id,
        None => {
            msg_error!("Cannot export morph track anim: no clip selected");
            return;
        }
    };

    let Some(model_path) = find_model_path(world) else {
        msg_error!("Cannot export morph track anim: no model loaded");
        return;
    };

    let library = world.resource::<ClipLibrary>();
    let source_clip = match library.get_source(clip_id) {
        Some(s) => s,
        None => {
            msg_error!("Cannot export morph track anim: clip not found");
            return;
        }
    };

    let curves = morph_track_curves(&source_clip.editable_clip);
    if curves.is_empty() {
        msg_error!(
            "Cannot export morph track anim: clip \"{}\" has no morph tracks",
            source_clip.name()
        );
        return;
    }

    let Some(export_dir) = create_unity_export_dir(&model_path) else {
        return;
    };
    let filename = format!("{}.anim", sanitize_filename(source_clip.name()));
    let path = export_dir.join(&filename);

    let yaml = write_unity_anim(source_clip.name(), &curves, source_clip.duration(), false);

    match fs::write(&path, yaml) {
        Ok(()) => msg_info!("Wrote morph track .anim to {}", path.display()),
        Err(error) => msg_error!("Failed to write {}: {}", path.display(), error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("happy_face"), "happy_face");
        assert_eq!(sanitize_filename("hello world"), "hello_world");
        assert_eq!(sanitize_filename("a-b_c"), "a-b_c");
        assert_eq!(sanitize_filename("test!@#name"), "test___name");
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn test_unity_export_dir() {
        let model_path = Path::new("/assets/models/character.fbx");
        let dir = unity_export_dir(model_path);
        assert_eq!(dir, PathBuf::from("/assets/models/character_unity"));
    }
}
