use cgmath::{Matrix4, SquareMatrix};

use crate::asset::AssetStorage;
use crate::ecs::resource::AnimationPlayback;
use crate::ecs::world::{
    AnimationState, Children, Entity, GlobalTransform, SkeletonRef, Transform, World,
};

pub fn animation_playback_system(
    playback: &mut AnimationPlayback,
    delta_time: f32,
    clip_duration: f32,
) {
    if !playback.playing || clip_duration <= 0.0 {
        return;
    }

    playback.time += delta_time * playback.speed;

    if playback.looping {
        playback.time = playback.time % clip_duration;
    } else if playback.time >= clip_duration {
        playback.time = clip_duration;
        playback.playing = false;
    }
}

pub fn transform_propagation_system(world: &mut World) {
    let root_entities = world.get_root_entities();

    fn propagate(world: &mut World, entity: Entity, parent_global: Matrix4<f32>) {
        let local_matrix = world
            .get_component::<Transform>(entity)
            .map(|t| t.to_matrix())
            .unwrap_or_else(Matrix4::identity);

        let global_matrix = parent_global * local_matrix;

        if let Some(gt) = world.get_component_mut::<GlobalTransform>(entity) {
            gt.0 = global_matrix;
        }

        let child_entities: Vec<Entity> = world
            .get_component::<Children>(entity)
            .map(|c| c.0.clone())
            .unwrap_or_default();

        for child in child_entities {
            propagate(world, child, global_matrix);
        }
    }

    for root in root_entities {
        propagate(world, root, Matrix4::identity());
    }
}

pub fn animation_time_system(world: &mut World, delta_time: f32, assets: &AssetStorage) {
    let animated_entities = world.query_animated();

    for entity in animated_entities {
        let Some(state) = world.get_component_mut::<AnimationState>(entity) else {
            continue;
        };

        if !state.playing {
            continue;
        }

        let Some(clip_id) = state.current_clip_id else {
            continue;
        };

        let duration = assets
            .animation_clips
            .values()
            .find(|c| c.clip_id == clip_id)
            .map(|c| c.clip.duration)
            .unwrap_or(1.0);

        state.time += delta_time * state.speed;

        if state.looping && duration > 0.0 {
            state.time = state.time % duration;
        } else if state.time > duration {
            state.time = duration;
            state.playing = false;
        }
    }
}

pub fn skeleton_animation_system(world: &mut World, assets: &mut AssetStorage) {
    let animated_entities = world.query_animated();

    struct AnimationData {
        skeleton_asset_id: crate::asset::AssetId,
        clip: crate::animation::AnimationClip,
        time: f32,
    }

    let mut animations_to_apply = Vec::new();

    for entity in animated_entities {
        let Some(state) = world.get_component::<AnimationState>(entity) else {
            continue;
        };
        let Some(skeleton_ref) = world.get_component::<SkeletonRef>(entity) else {
            continue;
        };

        let Some(clip_id) = state.current_clip_id else {
            continue;
        };

        let Some(clip_asset) = assets
            .animation_clips
            .values()
            .find(|c| c.clip_id == clip_id)
        else {
            continue;
        };

        animations_to_apply.push(AnimationData {
            skeleton_asset_id: skeleton_ref.0,
            clip: clip_asset.clip.clone(),
            time: state.time,
        });
    }

    for anim_data in animations_to_apply {
        if let Some(skeleton_asset) = assets.get_skeleton_mut(anim_data.skeleton_asset_id) {
            anim_data
                .clip
                .sample(anim_data.time, &mut skeleton_asset.skeleton);
        }
    }
}
