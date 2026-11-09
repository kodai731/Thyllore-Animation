use anyhow::Result;

use super::load::{read_model_file, replace_scene_model, ClipRegistration};
use crate::asset::AssetStorage;
use crate::ecs::resource::{ModelState, TimelineState};
use crate::ecs::systems::clip_library_systems::clip_library_register_loaded;
use crate::ecs::systems::rebuild_scene_acceleration_with_primitives;
use crate::ecs::world::World;
use crate::ecs::{GpuSceneContext, SceneState};
use crate::scene::{apply_scene_entities, capture_scene_entities, capture_scheduled_clips};
use crate::vulkanr::swapchain::RRSwapchain;

/// Replaces the model while keeping the scene's own entities and scheduled clips.
pub unsafe fn load_model_keeping_scene(
    path: &str,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    log!("Loading new model from: {}", path);
    let scene_entities = capture_scene_entities(world);
    let scheduled_clips = capture_scheduled_clips(world);

    let loaded = read_model_file(path).and_then(|(load_result, fbx_model)| {
        replace_scene_model(
            &load_result,
            path,
            gpu,
            swapchain,
            world,
            assets,
            ClipRegistration::FromModel,
            fbx_model,
        )
    });
    if let Err(error) = loaded {
        record_load_failure(world, &error);
        return Err(error);
    }

    record_loaded_model(world, path, format!("Loaded: {}", path));
    clip_library_register_loaded(world, assets, scheduled_clips);
    apply_scene_entities(world, assets, &scene_entities);

    let procedural_primitives = crate::hooks::gpu_primitive::collect_all(world);
    if !procedural_primitives.is_empty() {
        rebuild_scene_acceleration_with_primitives(gpu, world, assets, &procedural_primitives)?;
    }
    Ok(())
}

#[cfg(feature = "auto-rig")]
pub unsafe fn load_generated_mesh(
    glb_data: &[u8],
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    use crate::ecs::component::GlbSource;
    use crate::ecs::resource::GltfModelCache;
    use crate::scene::ModelReference;

    log!("Loading generated mesh from GLB ({} bytes)", glb_data.len());
    let loaded = crate::loader::gltf::load_gltf_from_slice(glb_data)
        .map(crate::loader::ModelLoadResult::from_gltf)
        .and_then(|load_result| {
            replace_scene_model(
                &load_result,
                ModelReference::GENERATED_MESH,
                gpu,
                swapchain,
                world,
                assets,
                ClipRegistration::FromModel,
                None,
            )
        });
    let parent_entity = match loaded {
        Ok(entity) => entity,
        Err(error) => {
            record_load_failure(world, &error);
            return Err(error);
        }
    };

    record_loaded_model(
        world,
        ModelReference::GENERATED_MESH,
        "Loaded: Generated Mesh".to_string(),
    );
    world.insert_resource(GltfModelCache::from_glb_data(glb_data.to_vec()));
    world.insert_component(parent_entity, GlbSource::InMemory(glb_data.to_vec()));
    Ok(())
}

fn record_loaded_model(world: &mut World, model_path: &str, load_status: String) {
    {
        let mut model_state = world.resource_mut::<ModelState>();
        model_state.model_path = model_path.to_string();
        model_state.load_status = load_status;
    }
    world.resource_mut::<TimelineState>().current_time = 0.0;
    world.resource_mut::<SceneState>().clear();
}

fn record_load_failure(world: &mut World, error: &anyhow::Error) {
    world.resource_mut::<ModelState>().load_status = format!("Error: {}", error);
}
