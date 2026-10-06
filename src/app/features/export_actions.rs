use std::path::Path;

use crate::animation::editable::EditableAnimationClip;
use crate::animation::Skeleton;
use crate::app::App;
use crate::ecs::resource::{
    ClipLibrary, FbxModelCache, GltfModelCache, HumanoidRigState, TimelineState,
};
use crate::ecs::systems::humanoid_bake_systems;

pub(crate) fn export_clip_fbx(app: &App, source_id: u64, path: &Path) {
    let Some((clip, skeleton)) = clip_with_skeleton(app, source_id) else {
        return;
    };

    let (fbx_model, has_skinned_meshes) = app
        .data
        .ecs_world
        .get_resource::<FbxModelCache>()
        .map(|cache| (cache.fbx_model().cloned(), cache.has_skinned_meshes()))
        .unwrap_or((None, false));

    let result = match fbx_model {
        Some(fbx_model) => {
            crate::exporter::fbx_exporter::export_full_fbx(&fbx_model, Some(&clip), &skeleton, path)
        }
        None => crate::exporter::fbx_animation::export_animation_fbx(
            &clip,
            &skeleton,
            path,
            has_skinned_meshes,
            crate::loader::fbx::fbx::FbxAxesInfo::default(),
            24.0,
        ),
    };

    match result {
        Ok(()) => msg_info!("FBX exported: {:?}", path),
        Err(e) => msg_error!("FBX export failed: {:?}", e),
    }
}

pub(crate) fn export_current_clip_fbx(app: &App, path: &Path) {
    let timeline_state = app
        .data
        .ecs_world
        .resource::<crate::ecs::resource::TimelineState>();
    let clip_id = timeline_state.current_clip_id;
    drop(timeline_state);
    if let Some(source_id) = clip_id {
        export_clip_fbx(app, source_id, path);
    }
}

pub(crate) fn export_clip_gltf(app: &App, source_id: u64, path: &Path) {
    let Some((clip, skeleton)) = clip_with_skeleton(app, source_id) else {
        return;
    };

    let source_bytes = app
        .data
        .ecs_world
        .get_resource::<GltfModelCache>()
        .and_then(|cache| resolve_glb_bytes(&cache));
    let Some(source_bytes) = source_bytes else {
        msg_error!("glTF export failed: no source glTF/GLB model loaded");
        return;
    };

    match crate::exporter::gltf::export_gltf_animation_from_bytes(
        &source_bytes,
        &clip,
        &skeleton,
        path,
    ) {
        Ok(()) => msg_info!("glTF exported: {:?}", path),
        Err(e) => msg_error!("glTF export failed: {:?}", e),
    }
}

pub(crate) fn export_clip_gltf_animation_only(app: &App, source_id: u64, path: &Path) {
    let Some((clip, skeleton)) = clip_with_skeleton(app, source_id) else {
        return;
    };

    match crate::exporter::gltf::export_gltf_animation_only(&clip, &skeleton, path) {
        Ok(()) => msg_info!("Animation-only glTF exported: {:?}", path),
        Err(e) => msg_error!("Animation-only glTF export failed: {:?}", e),
    }
}

pub(crate) fn export_model_gltf(app: &App, path: &Path) {
    let glb_bytes = app
        .data
        .ecs_world
        .get_resource::<GltfModelCache>()
        .and_then(|cache| resolve_glb_bytes(&cache));
    let Some(glb_bytes) = glb_bytes else {
        msg_error!("glTF export failed: no model data available");
        return;
    };

    match std::fs::write(path, &glb_bytes) {
        Ok(()) => msg_info!("Model exported: {:?}", path),
        Err(e) => msg_error!("Model export failed: {:?}", e),
    }
}

pub fn clip_with_skeleton(app: &App, source_id: u64) -> Option<(EditableAnimationClip, Skeleton)> {
    let library = app.data.ecs_world.resource::<ClipLibrary>();
    let clip = library.get(source_id).cloned()?;
    drop(library);

    let skeleton = app
        .data
        .ecs_assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone())?;

    let rig_state = app.data.ecs_world.get_resource::<HumanoidRigState>();
    let Some(rig) = rig_state.as_ref().and_then(|state| state.rig.as_ref()) else {
        return Some((clip, skeleton));
    };

    let fps = app
        .data
        .ecs_world
        .resource::<TimelineState>()
        .snap_settings
        .frame_rate
        .round() as u32;
    match humanoid_bake_systems::bake_to_bone_clip(&clip, &skeleton, rig, fps) {
        Ok(baked) => Some((baked, skeleton)),
        Err(e) => {
            msg_error!("Role clip bake failed: {:?}", e);
            None
        }
    }
}

fn resolve_glb_bytes(cache: &GltfModelCache) -> Option<Vec<u8>> {
    if let Some(ref data) = cache.glb_data {
        return Some(data.clone());
    }

    cache
        .source_path
        .as_ref()
        .and_then(|path| std::fs::read(path).ok())
}

pub(crate) fn extract_clip_name_from_path(path: &Path) -> String {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("clip");

    filename
        .strip_suffix(".anim.ron")
        .or_else(|| filename.strip_suffix(".ron"))
        .unwrap_or(filename)
        .to_string()
}
