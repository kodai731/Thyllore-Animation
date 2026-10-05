use anyhow::Result;

use super::clips::restore_batch_playback;
use super::gpu::release_model_gpu_resources;
use crate::asset::AssetStorage;
use crate::ecs::resource::{
    BakedRoleClips, BonePoseOverride, ClipLibrary, MeshAssets, NodeAssets, PoseApplyCache,
    TimelineState,
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
    let remaining_ids: Vec<_> = {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        clip_library.clear_model_clips();
        clip_library.all_clip_ids().copied().collect()
    };

    {
        let mut timeline_state = world.resource_mut::<TimelineState>();
        if let Some(current) = timeline_state.current_clip_id {
            if !remaining_ids.contains(&current) {
                timeline_state.current_clip_id = None;
            }
        }
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

    if let Some(mut baked) = world.get_resource_mut::<BakedRoleClips>() {
        *baked = BakedRoleClips::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{ClipSpace, EditableAnimationClip};
    use cgmath::Matrix4;
    use cgmath::SquareMatrix;

    fn make_world_with_model_resources() -> World {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(MeshAssets::default());
        world.insert_resource(NodeAssets::default());
        world.insert_resource(BonePoseOverride::default());
        world.insert_resource(BakedRoleClips::default());
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

    #[test]
    fn model_reset_keeps_role_clips_and_their_selection() {
        let mut world = make_world_with_model_resources();
        {
            let mut library = world.resource_mut::<ClipLibrary>();
            let mut role_clip = EditableAnimationClip::new(1, "role".to_string());
            role_clip.space = ClipSpace::HumanoidRole;
            library
                .source_clips
                .insert(1, crate::animation::editable::SourceClip::new(1, role_clip));
        }
        {
            let mut timeline = world.resource_mut::<TimelineState>();
            timeline.current_clip_id = Some(1);
        }

        reset_model_world_state(&mut world);

        let library = world.resource::<ClipLibrary>();
        assert_eq!(library.clip_count(), 1);
        assert!(library.get_source(1).is_some());
        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, Some(1));
    }

    #[test]
    fn model_reset_drops_bone_clips() {
        let mut world = make_world_with_model_resources();
        {
            let mut library = world.resource_mut::<ClipLibrary>();
            let bone_clip = EditableAnimationClip::new(2, "bone".to_string());
            library
                .source_clips
                .insert(2, crate::animation::editable::SourceClip::new(2, bone_clip));
        }
        {
            let mut timeline = world.resource_mut::<TimelineState>();
            timeline.current_clip_id = Some(2);
        }

        reset_model_world_state(&mut world);

        let library = world.resource::<ClipLibrary>();
        assert!(library.get_source(2).is_none());
        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, None);
    }
}
