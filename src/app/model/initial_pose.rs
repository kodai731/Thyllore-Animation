use super::clips::restore_batch_playback;
use crate::asset::AssetStorage;
use crate::ecs::resource::{NodeAssets, TimelineState};
use crate::ecs::systems::animation::apply::{
    apply_skinning_to_single_mesh, prepare_node_animation,
};
use crate::ecs::world::World;
use crate::ecs::{compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose};
use crate::loader::ModelLoadResult;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub(super) fn apply_initial_pose(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &mut GraphicsResources,
    load_result: &ModelLoadResult,
) -> Vec<usize> {
    if load_result.clips.is_empty() {
        return Vec::new();
    }

    log!("Applying initial pose (time=0) for animation...");
    reset_timeline_to_start(world);

    let mut updated_meshes = apply_initial_skinning(world, assets, graphics);

    let has_node_animation = !load_result.has_skinned_meshes && !graphics.meshes.is_empty();
    if has_node_animation {
        updated_meshes.extend(apply_initial_node_animation(
            world,
            assets,
            graphics,
            load_result.node_animation_scale,
        ));
    }

    log!("Initial pose applied successfully");
    updated_meshes
}

fn reset_timeline_to_start(world: &mut World) {
    {
        let mut timeline = world.resource_mut::<TimelineState>();
        timeline.playing = false;
        timeline.current_time = 0.0;
    }
    restore_batch_playback(world);
}

fn apply_initial_skinning(
    world: &World,
    assets: &AssetStorage,
    graphics: &mut GraphicsResources,
) -> Vec<usize> {
    let Some(skeleton_id) = graphics.meshes.first().and_then(|m| m.skeleton_id) else {
        return Vec::new();
    };
    let (current_time, looping) = {
        let timeline = world.resource::<TimelineState>();
        (timeline.current_time, timeline.looping)
    };

    let skeleton = assets.get_skeleton_by_skeleton_id(skeleton_id);
    let first_clip = assets.animation_clips.values().next().map(|a| &a.clip);
    let (Some(skeleton), Some(clip)) = (skeleton, first_clip) else {
        return Vec::new();
    };

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(clip, current_time, skeleton, &mut pose, looping);
    let globals = compute_pose_global_transforms(skeleton, &pose);

    (0..graphics.meshes.len())
        .filter(|&mesh_index| {
            apply_skinning_to_single_mesh(graphics, mesh_index, &globals, skeleton)
        })
        .collect()
}

fn apply_initial_node_animation(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &mut GraphicsResources,
    node_animation_scale: f32,
) -> Vec<usize> {
    let skeleton = graphics
        .meshes
        .first()
        .and_then(|m| m.skeleton_id)
        .and_then(|id| assets.get_skeleton_by_skeleton_id(id));
    let clip = assets.animation_clips.values().next().map(|a| &a.clip);
    let (Some(skeleton), Some(clip)) = (skeleton, clip) else {
        return Vec::new();
    };

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(clip, 0.0, skeleton, &mut pose, false);

    let mut node_assets = world.resource_mut::<NodeAssets>();
    prepare_node_animation(
        graphics,
        &mut node_assets.nodes,
        skeleton,
        &pose,
        node_animation_scale,
    )
}
