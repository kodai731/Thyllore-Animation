use cgmath::{Matrix4, SquareMatrix};

use crate::asset::AssetStorage;
use crate::ecs::resource::AnimationPlayback;
use crate::ecs::world::{
    Animator, Children, Entity, GlobalTransform, SkeletonRef, Transform, World,
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
        let Some(state) = world.get_component_mut::<Animator>(entity) else {
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

pub fn skeleton_animation_system(world: &mut World, assets: &AssetStorage) {
    use crate::ecs::{create_pose_from_rest, sample_clip_to_pose};

    let animated_entities = world.query_animated();

    for entity in animated_entities {
        let Some(state) = world.get_component::<Animator>(entity) else {
            continue;
        };
        let Some(skeleton_ref) = world.get_component::<SkeletonRef>(entity) else {
            continue;
        };

        let Some(clip_id) = state.current_clip_id else {
            continue;
        };

        let Some(skeleton_asset) = assets.get_skeleton(skeleton_ref.0) else {
            continue;
        };
        let Some(clip_asset) = assets
            .animation_clips
            .values()
            .find(|c| c.clip_id == clip_id)
        else {
            continue;
        };

        let skeleton = &skeleton_asset.skeleton;
        let mut pose = create_pose_from_rest(skeleton);
        sample_clip_to_pose(
            &clip_asset.clip,
            state.time,
            skeleton,
            &mut pose,
            state.looping,
        );
    }
}
