use crate::app::{features::export_actions, App};
use crate::ecs::resource::{
    AssetEditCommand, ClipLibrary, CommandQueue, EntityRemovalCommand, OutputCommand,
    SceneLoadCommand,
};
#[cfg(feature = "auto-rig")]
use crate::ecs::systems::phases::event_dispatch::ml::auto_rig::AutoRigEvent;

pub(crate) unsafe fn apply_queued_commands(app: &mut App) {
    for command in take_queued_commands::<EntityRemovalCommand>(app) {
        apply_entity_removal_command(app, command);
    }

    for command in take_queued_commands::<SceneLoadCommand>(app) {
        apply_scene_load_command(app, command);
    }

    for command in take_queued_commands::<AssetEditCommand>(app) {
        apply_asset_edit_command(app, command);
    }

    for command in take_queued_commands::<OutputCommand>(app) {
        apply_output_command(app, command);
    }
}

fn take_queued_commands<C: 'static>(app: &App) -> Vec<C> {
    app.data.ecs_world.resource_mut::<CommandQueue<C>>().take()
}

unsafe fn apply_entity_removal_command(app: &mut App, command: EntityRemovalCommand) {
    match command {
        EntityRemovalCommand::DeleteEntities { entities } => {
            if let Err(e) = app.delete_entities(&entities) {
                log_error!("Failed to delete entities: {:?}", e);
            }
        }
    }
}

unsafe fn apply_scene_load_command(app: &mut App, command: SceneLoadCommand) {
    match command {
        SceneLoadCommand::LoadModel { path } => {
            if let Err(e) = app.load_model(&path) {
                log_error!("Failed to load model: {:?}", e);
            }
        }

        SceneLoadCommand::LoadModelAdditive { path } => {
            if let Err(e) = app.load_model_additive(&path) {
                log_error!("Failed to add model: {:?}", e);
            }
        }

        #[cfg(feature = "auto-rig")]
        SceneLoadCommand::LoadModelFromMemory { glb_data, source } => {
            match app.load_model_from_glb(&glb_data) {
                Ok(()) => {
                    log!(
                        "SceneLoadCommand::LoadModelFromMemory: load OK, sending ModelLoadedFromMemory({:?})",
                        source
                    );
                    app.data
                        .ecs_world
                        .send_command(AutoRigEvent::ModelLoadedFromMemory { source });
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

        SceneLoadCommand::SpawnDebugPrimitive { kind } => {
            if let Err(e) = app.spawn_debug_primitive(kind) {
                log_error!("Failed to spawn debug primitive: {:?}", e);
            }
        }
    }
}

unsafe fn apply_asset_edit_command(app: &mut App, command: AssetEditCommand) {
    match command {
        AssetEditCommand::LoadClipFromFile { path } => {
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

        AssetEditCommand::LoadRecipeFromFile { path } => {
            if let Err(e) = crate::ecs::systems::motion_recipe_systems::apply_recipe_file(
                &mut app.data.ecs_world,
                &mut app.data.ecs_assets,
                &path,
            ) {
                msg_error!("Failed to load recipe {}: {:#}", path.display(), e);
            }
        }

        AssetEditCommand::AssignMaterialTexture { material, path } => {
            crate::ecs::systems::set_material_texture(
                &mut app.data.ecs_world,
                &material,
                Some(&path),
            );
        }
    }
}

unsafe fn apply_output_command(app: &mut App, command: OutputCommand) {
    match command {
        OutputCommand::TakeScreenshot => {
            log!("Taking screenshot...");
            let image_index = app.frame % crate::app::init::MAX_FRAMES_IN_FLIGHT;
            match app.save_screenshot(image_index) {
                Ok(path) => msg_info!("Screenshot saved: {}", path),
                Err(e) => log_error!("Screenshot failed: {:?}", e),
            }
        }

        #[cfg(debug_assertions)]
        OutputCommand::DebugShadowInfo => {
            crate::debugview::log_shadow_debug_info(
                &app.data.ecs_world,
                &app.data.raytracing,
                &app.data.graphics_resources,
            );
        }

        #[cfg(debug_assertions)]
        OutputCommand::DebugBillboardDepth => {
            crate::debugview::collect_and_log_billboard_debug(
                &app.data.ecs_world,
                &app.data.raytracing,
            );
        }

        OutputCommand::DumpDebugInfo => {
            app.dump_debug_info();
        }

        OutputCommand::CaptureNow(capture) => {
            app.capture_now(capture.as_ref());
        }

        OutputCommand::DumpAnimationDebug => {
            let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
            if let Err(e) = crate::ecs::systems::animation_debug_dump::dump_animation_debug(
                &app.data.ecs_world,
                &app.data.ecs_assets,
                &*clip_library,
            ) {
                log_warn!("Animation debug dump failed: {:?}", e);
            }
        }

        OutputCommand::SaveClipToFile { source_id, path } => {
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

        OutputCommand::SaveSpringBoneBake { baked_id, path } => {
            use crate::ecs::systems::clip_library_systems::clip_library_save_to_file;

            let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
            match clip_library_save_to_file(&clip_library, baked_id, &path) {
                Ok(()) => msg_info!("Saved spring bone bake to {:?}", path),
                Err(e) => msg_error!("Failed to save spring bone bake: {:?}", e),
            }
        }

        OutputCommand::ExportClipFbx { source_id, path } => {
            export_actions::export_clip_fbx(app, source_id, &path)
        }

        OutputCommand::ExportClipGltf { source_id, path } => {
            export_actions::export_clip_gltf(app, source_id, &path)
        }

        OutputCommand::ExportClipGltfAnimationOnly { source_id, path } => {
            export_actions::export_clip_gltf_animation_only(app, source_id, &path)
        }

        OutputCommand::ExportModelGltf { path } => export_actions::export_model_gltf(app, &path),
    }
}
