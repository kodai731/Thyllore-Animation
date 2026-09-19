use std::path::PathBuf;

use crate::app::App;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, ModelState, SpringBoneState};
use crate::ecs::DeferredAction;

pub(super) fn process_platform_file_events(
    events: &[UIEvent],
    app: &mut App,
) -> Vec<DeferredAction> {
    events
        .iter()
        .filter_map(|event| match event {
            UIEvent::ClipBrowserLoadFromFile => open_clip_load_dialog(),
            UIEvent::ClipBrowserSaveToFile(source_id) => open_clip_save_dialog(app, *source_id),
            UIEvent::ClipBrowserExportFbx(source_id) => {
                open_clip_export_dialog(app, *source_id, ClipExportFormat::Fbx)
            }
            UIEvent::ClipBrowserExportGltf(source_id) => {
                open_clip_export_dialog(app, *source_id, ClipExportFormat::Gltf)
            }
            UIEvent::ClipBrowserExportGltfAnimationOnly(source_id) => {
                open_clip_export_dialog(app, *source_id, ClipExportFormat::GltfAnimationOnly)
            }
            UIEvent::ExportModelGltf => open_model_export_dialog(app),
            UIEvent::SpringBoneSaveBake => open_spring_bone_save_dialog(app),
            _ => None,
        })
        .collect()
}

fn open_clip_load_dialog() -> Option<DeferredAction> {
    let path = rfd::FileDialog::new()
        .add_filter("Animation RON", &["anim.ron", "ron"])
        .pick_file()?;

    Some(DeferredAction::LoadClipFromFile { path })
}

fn open_clip_save_dialog(app: &App, source_id: u64) -> Option<DeferredAction> {
    let current_name = clip_name(app, source_id).unwrap_or_else(|| "clip".to_string());

    let path = rfd::FileDialog::new()
        .add_filter("Animation RON", &["anim.ron", "ron"])
        .set_file_name(format!("{}.anim.ron", current_name))
        .save_file()?;

    Some(DeferredAction::SaveClipToFile { source_id, path })
}

#[derive(Clone, Copy)]
enum ClipExportFormat {
    Fbx,
    Gltf,
    GltfAnimationOnly,
}

fn open_clip_export_dialog(
    app: &App,
    source_id: u64,
    format: ClipExportFormat,
) -> Option<DeferredAction> {
    let clip_name = clip_name(app, source_id)?;
    let (filter_name, extension, default_filename) = match format {
        ClipExportFormat::Fbx => ("FBX Binary", "fbx", format!("{}.fbx", clip_name)),
        ClipExportFormat::Gltf => ("glTF Binary", "glb", format!("{}.glb", clip_name)),
        ClipExportFormat::GltfAnimationOnly => {
            ("glTF Binary", "glb", format!("{}_anim_only.glb", clip_name))
        }
    };

    let path = save_file_dialog(filter_name, extension, &default_filename)?;

    Some(match format {
        ClipExportFormat::Fbx => DeferredAction::ExportClipFbx { source_id, path },
        ClipExportFormat::Gltf => DeferredAction::ExportClipGltf { source_id, path },
        ClipExportFormat::GltfAnimationOnly => {
            DeferredAction::ExportClipGltfAnimationOnly { source_id, path }
        }
    })
}

fn open_model_export_dialog(app: &App) -> Option<DeferredAction> {
    let model_path = app
        .data
        .ecs_world
        .resource::<ModelState>()
        .model_path
        .clone();
    let model_stem = std::path::Path::new(&model_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");

    let path = save_file_dialog("glTF Binary", "glb", &format!("{}.glb", model_stem))?;

    Some(DeferredAction::ExportModelGltf { path })
}

fn open_spring_bone_save_dialog(app: &App) -> Option<DeferredAction> {
    let baked_id = app
        .data
        .ecs_world
        .resource::<SpringBoneState>()
        .baked_clip_source_id?;

    let path = rfd::FileDialog::new()
        .add_filter("Animation RON", &["anim.ron", "ron"])
        .set_file_name("spring_baked.anim.ron")
        .save_file()?;

    Some(DeferredAction::SaveSpringBoneBake { baked_id, path })
}

fn clip_name(app: &App, source_id: u64) -> Option<String> {
    app.data
        .ecs_world
        .resource::<ClipLibrary>()
        .get(source_id)
        .map(|clip| clip.name.clone())
}

fn save_file_dialog(filter_name: &str, extension: &str, default_filename: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(filter_name, &[extension])
        .set_file_name(default_filename)
        .save_file()
}
