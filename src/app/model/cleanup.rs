use anyhow::Result;

use super::clips::restore_batch_playback;
use super::gpu::release_model_gpu_resources;
use crate::asset::AssetStorage;
use crate::ecs::resource::{BonePoseOverride, ClipLibrary, MeshAssets, NodeAssets, TimelineState};
use crate::ecs::world::World;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

pub(super) unsafe fn reset_scene_model(
    device: &RRDevice,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    log!("Cleaning up model resources...");

    release_model_gpu_resources(device, graphics, raytracing)?;
    reset_model_world_state(world);
    world.clear();
    assets.clear();

    log!("Model resources cleaned up");
    Ok(())
}

fn reset_model_world_state(world: &mut World) {
    world.resource_mut::<ClipLibrary>().clear();

    {
        let mut timeline_state = world.resource_mut::<TimelineState>();
        timeline_state.current_clip_id = None;
        timeline_state.current_time = 0.0;
        timeline_state.selected_keyframes.clear();
        timeline_state.expanded_tracks.clear();
    }
    restore_batch_playback(world);

    world.resource_mut::<MeshAssets>().meshes.clear();
    world.resource_mut::<NodeAssets>().nodes.clear();
    world.resource_mut::<BonePoseOverride>().overrides.clear();
}
