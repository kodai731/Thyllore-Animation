use std::fs;
use std::path::{Path, PathBuf};

use thyllore_anim_core::editable::systems::morph_sample::sample_morph_tracks;
use thyllore_exporter_core::systems::unity::anim::write_unity_anim;
use thyllore_exporter_core::systems::unity::curves::{morph_track_curves, write_expression_anim};

use crate::animation::editable::EditableAnimationClip;
use crate::asset::AssetStorage;
use crate::ecs::resource::{ClipLibrary, ExpressionLibraryState, TimelineState};
use crate::ecs::systems::avatar_setup_systems::{
    export_avatar_sidecar, find_expression_morph, find_model_path,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const MORPH_TRACK_SAMPLE_COUNT: usize = 21;

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

pub fn export_unity_avatar(world: &World, assets: &AssetStorage, graphics: &GraphicsResources) {
    export_avatar_sidecar(world, assets, graphics);
    export_expression_anims(world, assets, graphics);
    export_morph_track_anim(world);
}

fn export_expression_anims(world: &World, assets: &AssetStorage, graphics: &GraphicsResources) {
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

struct MorphTrackClip {
    clip: EditableAnimationClip,
    name: String,
    duration: f32,
    anim_path: PathBuf,
}

fn find_current_morph_track_clip(world: &World) -> Result<MorphTrackClip, String> {
    let clip_id = world
        .resource::<TimelineState>()
        .current_clip_id
        .ok_or("no clip selected")?;
    let model_path = find_model_path(world).ok_or("no model loaded")?;

    let library = world.resource::<ClipLibrary>();
    let source_clip = library.get_source(clip_id).ok_or("clip not found")?;
    if source_clip.editable_clip.morph_tracks.is_empty() {
        return Err(format!(
            "clip \"{}\" has no morph tracks",
            source_clip.name()
        ));
    }

    let filename = format!("{}.anim", sanitize_filename(source_clip.name()));
    Ok(MorphTrackClip {
        clip: source_clip.editable_clip.clone(),
        name: source_clip.name().to_string(),
        duration: source_clip.duration(),
        anim_path: unity_export_dir(Path::new(&model_path)).join(filename),
    })
}

fn write_export_file(path: &Path, content: String) -> Result<(), String> {
    let export_dir = path.parent().ok_or("export path has no directory")?;
    fs::create_dir_all(export_dir)
        .and_then(|()| fs::write(path, content))
        .map_err(|error| format!("{}: {}", path.display(), error))
}

fn export_morph_track_anim(world: &World) {
    let morph_clip = match find_current_morph_track_clip(world) {
        Ok(morph_clip) => morph_clip,
        Err(reason) => {
            msg_info!("Skipped current clip .anim: {}", reason);
            return;
        }
    };

    let curves = morph_track_curves(&morph_clip.clip);
    let yaml = write_unity_anim(&morph_clip.name, &curves, morph_clip.duration, false);

    match write_export_file(&morph_clip.anim_path, yaml) {
        Ok(()) => msg_info!(
            "Wrote morph track .anim to {}",
            morph_clip.anim_path.display()
        ),
        Err(error) => msg_error!("Failed to write {}", error),
    }
}

/// Writes what the engine evaluates for the exported morph clip, next to the `.anim`, so an importer
/// of that file can be compared against it.
pub fn dump_morph_track_samples(world: &World) {
    let morph_clip = match find_current_morph_track_clip(world) {
        Ok(morph_clip) => morph_clip,
        Err(reason) => {
            msg_error!("Cannot dump morph track samples: {}", reason);
            return;
        }
    };

    let samples: Vec<serde_json::Value> = (0..MORPH_TRACK_SAMPLE_COUNT)
        .map(|index| {
            let time = morph_clip.duration * index as f32 / (MORPH_TRACK_SAMPLE_COUNT - 1) as f32;
            let weights: Vec<serde_json::Value> = sample_morph_tracks(&morph_clip.clip, time)
                .into_iter()
                .map(|sample| {
                    serde_json::json!({
                        "source_mesh": sample.source_mesh,
                        "channel": sample.channel,
                        "weight": sample.weight,
                    })
                })
                .collect();
            serde_json::json!({ "time": time, "weights": weights })
        })
        .collect();
    let dump = serde_json::json!({
        "clip": morph_clip.name,
        "anim_file": morph_clip.anim_path.file_name().map(|name| name.to_string_lossy()),
        "duration": morph_clip.duration,
        "samples": samples,
    });

    let samples_path = morph_clip.anim_path.with_extension("samples.json");
    match write_export_file(&samples_path, format!("{:#}", dump)) {
        Ok(()) => msg_info!("Wrote morph track samples to {}", samples_path.display()),
        Err(error) => msg_error!("Failed to write {}", error),
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
