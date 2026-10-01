use anyhow::Result;

use super::clips::restore_batch_playback;
use super::gpu::release_model_gpu_resources;
use crate::asset::AssetStorage;
use crate::ecs::resource::{
    BonePoseOverride, ClipLibrary, MeshAssets, NodeAssets, PoseApplyCache, TimelineState,
};
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

    if let Some(mut pose_apply_cache) = world.get_resource_mut::<PoseApplyCache>() {
        *pose_apply_cache = PoseApplyCache::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Matrix4;
    use cgmath::SquareMatrix;

    fn make_world_with_model_resources() -> World {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(MeshAssets::default());
        world.insert_resource(NodeAssets::default());
        world.insert_resource(BonePoseOverride::default());
        world
    }

    #[test]
    fn reset_model_world_state_forgets_applied_poses() {
        let mut world = make_world_with_model_resources();
        let mut cache = PoseApplyCache::default();
        cache.skinned_cache.insert(6, vec![Matrix4::identity()]);
        cache.node_cache.insert(9, (Matrix4::identity(), 1.0));
        world.insert_resource(cache);

        reset_model_world_state(&mut world);

        let cache = world.resource::<PoseApplyCache>();
        assert!(cache.skinned_cache.is_empty());
        assert!(cache.node_cache.is_empty());
    }

    #[test]
    fn reset_model_world_state_tolerates_missing_pose_cache() {
        let mut world = make_world_with_model_resources();

        reset_model_world_state(&mut world);

        assert!(!world.contains_resource::<PoseApplyCache>());
    }
}
