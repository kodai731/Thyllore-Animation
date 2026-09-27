use thyllore_anim_core::editable::components::morph_track::MorphTrackSample;
use thyllore_anim_core::editable::systems::morph_sample::sample_morph_tracks;

use crate::animation::editable::{
    clip_recalculate_duration, curve_add_keyframe, EditableAnimationClip,
};
use crate::asset::AssetStorage;
use crate::ecs::component::{ClipSchedule, MorphWeights};
use crate::ecs::resource::{ClipLibrary, MorphTrackPlayback, TimelineState};
use crate::ecs::world::{Entity, Parent, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::morph_weight_systems::find_mesh_morph;
use super::scalar_clip_systems::{ensure_entity_clip_named, find_entity_clip_id};

const EXPRESSION_CLIP_NAME: &str = "Expression";
const SAME_KEY_TIME_EPSILON: f32 = 1e-6;

pub fn key_morph_weights(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
    entity: Entity,
) {
    let Some(root) = world.get_component::<Parent>(entity).map(|parent| parent.0) else {
        return;
    };
    let Some(morph) = find_mesh_morph(world, entity, assets, graphics) else {
        return;
    };
    let source_mesh = morph.source_mesh.clone();
    let channel_names: Vec<String> = morph
        .channels
        .iter()
        .map(|channel| channel.name.clone())
        .collect();
    let Some(weights) = world
        .get_component::<MorphWeights>(entity)
        .map(|morph_weights| morph_weights.weights.clone())
    else {
        return;
    };
    let time = world
        .get_resource::<TimelineState>()
        .map_or(0.0, |timeline| timeline.current_time);

    let clip_id = ensure_entity_clip_named(world, assets, root, EXPRESSION_CLIP_NAME);
    let mut clip_library = world.resource_mut::<ClipLibrary>();
    let Some(clip) = clip_library.get_mut(clip_id) else {
        return;
    };

    for (channel, weight) in channels_to_key(&channel_names, &weights, clip, &source_mesh) {
        insert_morph_key(clip, &source_mesh, &channel, time, weight);
    }
    clip_recalculate_duration(clip);
}

fn channels_to_key(
    channel_names: &[String],
    weights: &[f32],
    clip: &EditableAnimationClip,
    source_mesh: &str,
) -> Vec<(String, f32)> {
    channel_names
        .iter()
        .zip(weights)
        .filter(|(name, weight)| {
            **weight != 0.0 || clip.get_morph_track(source_mesh, name).is_some()
        })
        .map(|(name, weight)| (name.clone(), *weight))
        .collect()
}

fn insert_morph_key(
    clip: &mut EditableAnimationClip,
    source_mesh: &str,
    channel: &str,
    time: f32,
    weight: f32,
) {
    let curve = &mut clip.get_or_add_morph_track(source_mesh, channel).curve;
    if let Some(existing) = curve
        .keyframes
        .iter_mut()
        .find(|key| (key.time - time).abs() < SAME_KEY_TIME_EPSILON)
    {
        existing.value = weight;
    } else {
        curve_add_keyframe(curve, time, weight);
    }
}

pub fn evaluate_morph_tracks(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    let Some((time, playing)) = world
        .get_resource::<TimelineState>()
        .map(|timeline| (timeline.current_time, timeline.playing))
    else {
        return;
    };
    {
        let Some(mut playback) = world.get_resource_mut::<MorphTrackPlayback>() else {
            return;
        };
        if !playing && playback.last_time == Some(time) {
            return;
        }
        playback.last_time = Some(time);
    }

    for (root, samples) in collect_morph_samples_by_root(world, time) {
        apply_morph_samples_to_children(world, assets, graphics, root, &samples);
    }
}

fn collect_morph_samples_by_root(world: &World, time: f32) -> Vec<(Entity, Vec<MorphTrackSample>)> {
    let Some(clip_library) = world.get_resource::<ClipLibrary>() else {
        return Vec::new();
    };

    world
        .iter_components::<ClipSchedule>()
        .filter_map(|(root, _)| {
            let clip = clip_library.get(find_entity_clip_id(world, root)?)?;
            (!clip.morph_tracks.is_empty()).then(|| (root, sample_morph_tracks(clip, time)))
        })
        .collect()
}

fn apply_morph_samples_to_children(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    root: Entity,
    samples: &[MorphTrackSample],
) {
    let children: Vec<Entity> = world
        .iter_components::<MorphWeights>()
        .map(|(child, _)| child)
        .filter(|&child| world.get_component::<Parent>(child).map(|parent| parent.0) == Some(root))
        .collect();

    for child in children {
        let Some(morph) = find_mesh_morph(world, child, assets, graphics) else {
            continue;
        };
        let channel_weights: Vec<(usize, f32)> = samples
            .iter()
            .filter(|sample| sample.source_mesh == morph.source_mesh)
            .filter_map(|sample| {
                morph
                    .channels
                    .iter()
                    .position(|channel| channel.name == sample.channel)
                    .map(|index| (index, sample.weight.clamp(0.0, 1.0)))
            })
            .collect();

        let Some(morph_weights) = world.get_component_mut::<MorphWeights>(child) else {
            continue;
        };
        for (index, weight) in channel_weights {
            if let Some(slot) = morph_weights.weights.get_mut(index) {
                *slot = weight;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(channels: &[&str]) -> Vec<String> {
        channels.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn channels_to_key_takes_only_non_zero_weights_on_empty_clip() {
        let clip = EditableAnimationClip::new(0, EXPRESSION_CLIP_NAME.to_string());

        let keys = channels_to_key(
            &names(&["smile", "blink", "angry"]),
            &[0.5, 0.0, 1.0],
            &clip,
            "face",
        );

        assert_eq!(
            keys,
            vec![("smile".to_string(), 0.5), ("angry".to_string(), 1.0)]
        );
    }

    #[test]
    fn channels_to_key_keeps_zero_weight_channels_that_already_have_a_track() {
        let mut clip = EditableAnimationClip::new(0, EXPRESSION_CLIP_NAME.to_string());
        insert_morph_key(&mut clip, "face", "blink", 0.0, 1.0);

        let keys = channels_to_key(&names(&["smile", "blink"]), &[0.0, 0.0], &clip, "face");

        assert_eq!(keys, vec![("blink".to_string(), 0.0)]);
    }

    #[test]
    fn channels_to_key_ignores_tracks_of_another_source_mesh() {
        let mut clip = EditableAnimationClip::new(0, EXPRESSION_CLIP_NAME.to_string());
        insert_morph_key(&mut clip, "body", "blink", 0.0, 1.0);

        let keys = channels_to_key(&names(&["blink"]), &[0.0], &clip, "face");

        assert!(keys.is_empty());
    }

    #[test]
    fn insert_morph_key_overwrites_key_at_same_time() {
        let mut clip = EditableAnimationClip::new(0, EXPRESSION_CLIP_NAME.to_string());
        insert_morph_key(&mut clip, "face", "smile", 1.0, 0.2);
        insert_morph_key(&mut clip, "face", "smile", 1.0, 0.8);

        let track = clip
            .get_morph_track("face", "smile")
            .expect("smile track keyed");
        assert_eq!(track.curve.keyframes.len(), 1);
        assert_eq!(track.curve.keyframes[0].value, 0.8);
    }
}
