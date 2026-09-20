use crate::app::{features::export_actions, App};
#[cfg(feature = "auto-rig")]
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{AppCommand, ClipLibrary};
#[cfg(feature = "auto-rig")]
use crate::ecs::UIEventQueue;

pub(crate) unsafe fn apply_app_command(app: &mut App, command: AppCommand) {
    match command {
        AppCommand::LoadModel { path } => {
            if let Err(e) = app.load_model(&path) {
                log_error!("Failed to load model: {:?}", e);
            }
        }

        AppCommand::LoadModelAdditive { path } => {
            if let Err(e) = app.load_model_additive(&path) {
                log_error!("Failed to add model: {:?}", e);
            }
        }

        AppCommand::SpawnDebugPrimitive { kind } => {
            if let Err(e) = app.spawn_debug_primitive(kind) {
                log_error!("Failed to spawn debug primitive: {:?}", e);
            }
        }

        AppCommand::DeleteEntities { entities } => {
            if let Err(e) = app.delete_entities(&entities) {
                log_error!("Failed to delete entities: {:?}", e);
            }
        }

        AppCommand::TakeScreenshot => {
            log!("Taking screenshot...");
            let image_index = app.frame % crate::app::init::MAX_FRAMES_IN_FLIGHT;
            match app.save_screenshot(image_index) {
                Ok(path) => msg_info!("Screenshot saved: {}", path),
                Err(e) => log_error!("Screenshot failed: {:?}", e),
            }
        }

        #[cfg(debug_assertions)]
        AppCommand::DebugShadowInfo => {
            crate::debugview::log_shadow_debug_info(
                &app.data.ecs_world,
                &app.data.raytracing,
                &app.data.graphics_resources,
            );
        }

        #[cfg(debug_assertions)]
        AppCommand::DebugBillboardDepth => {
            crate::debugview::collect_and_log_billboard_debug(
                &app.data.ecs_world,
                &app.data.raytracing,
            );
        }

        AppCommand::DumpDebugInfo => {
            app.dump_debug_info();
        }

        AppCommand::CaptureNow(capture) => {
            app.capture_now(capture.as_ref());
        }

        AppCommand::DumpAnimationDebug => {
            let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
            if let Err(e) = crate::ecs::systems::animation_debug_dump::dump_animation_debug(
                &app.data.ecs_world,
                &app.data.ecs_assets,
                &*clip_library,
            ) {
                log_warn!("Animation debug dump failed: {:?}", e);
            }
        }

        AppCommand::LoadClipFromFile { path } => {
            let bone_name_to_id = app
                .data
                .ecs_assets
                .skeletons
                .values()
                .next()
                .map(|sa| sa.skeleton.bone_name_to_id.clone());

            let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
            match crate::ecs::systems::clip_library_systems::clip_library_load_from_file(
                &mut clip_library,
                &mut app.data.ecs_assets,
                &path,
                bone_name_to_id.as_ref(),
            ) {
                Ok(_) => {}
                Err(e) => msg_error!("Failed to load clip: {:?}", e),
            }
        }

        AppCommand::SaveClipToFile { source_id, path } => {
            use crate::ecs::systems::clip_library_systems::{
                clip_library_save_to_file, clip_library_update_save_metadata,
            };

            let new_name = export_actions::extract_clip_name_from_path(&path);
            let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
            clip_library_update_save_metadata(
                &mut clip_library,
                source_id,
                new_name.clone(),
                &path,
            );

            match clip_library_save_to_file(&clip_library, source_id, &path) {
                Ok(()) => msg_info!("Saved clip '{}' to {:?}", new_name, path),
                Err(e) => msg_error!("Failed to save clip: {:?}", e),
            }
        }

        AppCommand::SaveSpringBoneBake { baked_id, path } => {
            use crate::ecs::systems::clip_library_systems::clip_library_save_to_file;

            let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
            match clip_library_save_to_file(&clip_library, baked_id, &path) {
                Ok(()) => msg_info!("Saved spring bone bake to {:?}", path),
                Err(e) => msg_error!("Failed to save spring bone bake: {:?}", e),
            }
        }

        AppCommand::ExportClipFbx { source_id, path } => {
            export_actions::export_clip_fbx(app, source_id, &path)
        }

        AppCommand::ExportClipGltf { source_id, path } => {
            export_actions::export_clip_gltf(app, source_id, &path)
        }

        AppCommand::ExportClipGltfAnimationOnly { source_id, path } => {
            export_actions::export_clip_gltf_animation_only(app, source_id, &path)
        }

        AppCommand::ExportModelGltf { path } => export_actions::export_model_gltf(app, &path),

        #[cfg(feature = "auto-rig")]
        AppCommand::LoadModelFromMemory { glb_data, source } => {
            match app.load_model_from_glb(&glb_data) {
                Ok(()) => {
                    log!(
                        "AppCommand::LoadModelFromMemory: load OK, sending ModelLoadedFromMemory({:?})",
                        source
                    );
                    let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                    ui_events.send(UIEvent::ModelLoadedFromMemory { source });
                }
                Err(e) => {
                    log_error!("Failed to load generated mesh: {}", e);
                    let mut state = app
                        .data
                        .ecs_world
                        .resource_mut::<crate::ecs::resource::TextToMeshState>();
                    state.status = crate::ecs::resource::TextToMeshStatus::Error;
                    state.error_message = Some(format!("Failed to load GLB: {}", e));
                }
            }
        }
    }
}
