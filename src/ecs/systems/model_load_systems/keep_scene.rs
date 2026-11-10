use anyhow::Result;

use super::replace::{read_model_file, replace_scene_model};
use crate::animation::editable::EditableAnimationClip;
use crate::asset::AssetStorage;
use crate::ecs::resource::{ModelState, TimelineState};
use crate::ecs::systems::clip_library_systems::clip_library_register_loaded;
use crate::ecs::systems::rebuild_scene_acceleration;
use crate::ecs::world::World;
use crate::ecs::{GpuSceneContext, SceneState};
use crate::scene::{
    apply_scene_entities, capture_scene_entities, capture_scheduled_clips, SceneEntity,
};
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
    let (load_result, fbx_model) = match read_model_file(path) {
        Ok(model_file) => model_file,
        Err(error) => return report_load_result(world, Err(error)),
    };

    let kept_scene = capture_kept_scene(world);
    let replaced =
        replace_scene_model(&load_result, path, gpu, swapchain, world, assets, fbx_model)
            .map(|_| record_loaded_model(world, path, format!("Loaded: {}", path)));
    let restored = restore_kept_scene(kept_scene, gpu, world, assets);

    report_load_result(world, replaced.and(restored))
}

/// Replaces the model by an in-memory GLB while keeping the scene's own entities and scheduled clips.
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
    let load_result = match crate::loader::gltf::load_gltf_from_slice(glb_data) {
        Ok(gltf) => crate::loader::ModelLoadResult::from_gltf(gltf),
        Err(error) => return report_load_result(world, Err(error)),
    };

    let kept_scene = capture_kept_scene(world);
    let replaced = replace_scene_model(
        &load_result,
        ModelReference::GENERATED_MESH,
        gpu,
        swapchain,
        world,
        assets,
        None,
    )
    .map(|parent_entity| {
        record_loaded_model(
            world,
            ModelReference::GENERATED_MESH,
            "Loaded: Generated Mesh".to_string(),
        );
        world.insert_resource(GltfModelCache::from_glb_data(glb_data.to_vec()));
        world.insert_component(parent_entity, GlbSource::InMemory(glb_data.to_vec()));
    });
    let restored = restore_kept_scene(kept_scene, gpu, world, assets);

    report_load_result(world, replaced.and(restored))
}

struct KeptScene {
    entities: Vec<SceneEntity>,
    scheduled_clips: Vec<EditableAnimationClip>,
}

fn capture_kept_scene(world: &World) -> KeptScene {
    KeptScene {
        entities: capture_scene_entities(world),
        scheduled_clips: capture_scheduled_clips(world),
    }
}

/// Runs whether or not the replacement succeeded, so a failed load still leaves the scene's entities.
unsafe fn restore_kept_scene(
    kept_scene: KeptScene,
    gpu: &mut GpuSceneContext,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    clip_library_register_loaded(world, assets, kept_scene.scheduled_clips);
    apply_scene_entities(world, assets, &kept_scene.entities);

    rebuild_scene_acceleration(gpu, world, assets)
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

fn report_load_result(world: &mut World, loaded: Result<()>) -> Result<()> {
    if let Err(error) = &loaded {
        world.resource_mut::<ModelState>().load_status = format!("Error: {}", error);
    }
    loaded
}
