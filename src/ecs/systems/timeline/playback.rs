use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::world::World;

/// Range the timeline spans when no clip supplies one, so the ruler, the transport and
/// playback all agree on how far the playhead may travel.
pub const TIMELINE_FALLBACK_DURATION_SECONDS: f32 = 5.0;

/// Range the playhead travels over. Always positive: scrubbing and playback stay usable
/// before a clip exists. Covers both the selected clip's own duration and the furthest
/// scheduled instance end, so a drag-extended instance of a short (or empty) clip can
/// still be scrubbed to its end.
pub fn timeline_effective_duration(
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
) -> f32 {
    let clip_duration = timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
        .map(|c| c.duration)
        .unwrap_or(0.0);

    let duration = clip_duration.max(timeline_state.schedule_extent_seconds);
    if duration > 0.0 {
        duration
    } else {
        TIMELINE_FALLBACK_DURATION_SECONDS
    }
}

/// Furthest end time any scheduled clip instance reaches, across every entity.
/// Muted instances count too: their blocks stay visible on the timeline, so the
/// ruler must still reach them.
pub fn schedule_extent_seconds(world: &World) -> f32 {
    world
        .component_entities::<ClipSchedule>()
        .iter()
        .filter_map(|&entity| world.get_component::<ClipSchedule>(entity))
        .flat_map(|schedule| schedule.instances.iter().map(|i| i.end_time()))
        .fold(0.0, f32::max)
}

pub fn timeline_update(
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
    delta_time: f32,
) {
    if !timeline_state.playing {
        return;
    }

    let duration = timeline_effective_duration(timeline_state, clip_library);
    let new_time = timeline_state.current_time + delta_time * timeline_state.speed;

    if timeline_state.looping {
        timeline_state.current_time = new_time % duration;
    } else if new_time >= duration {
        timeline_state.current_time = duration;
        timeline_state.playing = false;
    } else {
        timeline_state.current_time = new_time;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::ClipInstance;
    use crate::ecs::systems::timeline::test_support::clip_with_three_keys;

    #[test]
    fn timeline_advances_without_a_clip() {
        let library = ClipLibrary::new();
        let mut state = TimelineState::new();
        state.playing = true;

        timeline_update(&mut state, &library, 0.5);

        assert!((state.current_time - 0.5).abs() < 1e-5);
    }

    #[test]
    fn timeline_without_a_clip_loops_at_the_fallback_duration() {
        let library = ClipLibrary::new();
        let mut state = TimelineState::new();
        state.playing = true;
        state.looping = true;
        state.current_time = TIMELINE_FALLBACK_DURATION_SECONDS - 0.25;

        timeline_update(&mut state, &library, 0.5);

        assert!((state.current_time - 0.25).abs() < 1e-5);
    }

    #[test]
    fn timeline_without_a_clip_stops_at_the_fallback_duration_when_not_looping() {
        let library = ClipLibrary::new();
        let mut state = TimelineState::new();
        state.playing = true;
        state.looping = false;
        state.current_time = TIMELINE_FALLBACK_DURATION_SECONDS - 0.25;

        timeline_update(&mut state, &library, 0.5);

        assert!((state.current_time - TIMELINE_FALLBACK_DURATION_SECONDS).abs() < 1e-5);
        assert!(!state.playing);
    }

    #[test]
    fn timeline_uses_the_clip_duration_when_one_is_selected() {
        let (mut state, library) = clip_with_three_keys();
        let clip_duration = library.get(1).unwrap().duration;
        assert!(clip_duration > 0.0 && clip_duration < TIMELINE_FALLBACK_DURATION_SECONDS);

        assert!(
            (timeline_effective_duration(&state, &library) - clip_duration).abs() < 1e-5,
            "effective duration must follow the clip, not the fallback"
        );

        state.playing = true;
        state.looping = true;
        state.current_time = clip_duration - 0.1;
        timeline_update(&mut state, &library, 0.2);

        assert!((state.current_time - 0.1).abs() < 1e-5);
    }

    #[test]
    fn timeline_extends_to_the_schedule_extent_beyond_the_clip_duration() {
        let (mut state, library) = clip_with_three_keys();
        let clip_duration = library.get(1).unwrap().duration;

        state.schedule_extent_seconds = clip_duration + 2.0;
        assert!(
            (timeline_effective_duration(&state, &library) - (clip_duration + 2.0)).abs() < 1e-5
        );

        state.playing = true;
        state.looping = true;
        state.current_time = clip_duration + 1.9;
        timeline_update(&mut state, &library, 0.2);
        assert!((state.current_time - 0.1).abs() < 1e-5);
    }

    #[test]
    fn empty_clip_with_extended_instance_uses_the_extent_not_the_fallback() {
        let library = ClipLibrary::new();
        let mut state = TimelineState::new();
        state.schedule_extent_seconds = 8.0;

        assert!((timeline_effective_duration(&state, &library) - 8.0).abs() < 1e-5);
    }

    #[test]
    fn schedule_extent_covers_every_entity_and_counts_muted_instances() {
        let mut world = World::new();

        let a = world.spawn();
        let mut schedule_a = ClipSchedule::new();
        schedule_a.instances.push(ClipInstance::new(1, 10, 0.0));
        schedule_a.instances[0].clip_out = 3.0;
        world.insert_component(a, schedule_a);

        let b = world.spawn();
        let mut schedule_b = ClipSchedule::new();
        schedule_b.instances.push(ClipInstance::new(1, 11, 0.0));
        schedule_b.instances[0].start_time = 1.0;
        schedule_b.instances[0].clip_out = 4.5;
        schedule_b.instances[0].muted = true;
        world.insert_component(b, schedule_b);

        assert!((schedule_extent_seconds(&world) - 5.5).abs() < 1e-5);
    }
}
