use crate::animation::editable::{
    BezierHandle, CurveExtrapolation, InterpolationType, KeyframeId, PropertyType, SourceClipId,
    TangentContinuity, TangentType, TangentWeightMode,
};
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{scalar_domain_for_entity, ClipSchedule};
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{
    BonePoseOverride, ClipLibrary, ClipPreview, CurveTrackRef, SelectedKeyframe, SelectionModifier,
    TimelineState,
};
use crate::ecs::systems::clip_schedule_systems::clip_schedule_switch_source;
use crate::ecs::systems::timeline::{
    timeline_apply_selection, timeline_select_clip, timeline_set_keyframe_selection,
    timeline_toggle_track_expanded, timeline_zoom_in, timeline_zoom_out,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::super::spring_bone::transition_to_baked_override_if_needed;
use super::bone_set_key::dispatch_bone_set_key_events;
use super::clipboard::dispatch_keyframe_clipboard_events;
use super::curve_buffer::dispatch_curve_buffer_events;
use super::history::ClipSnapshot;
use super::keyframe::apply_keyframe_edits;
use super::tangent::apply_tangent_edits;
use super::ExtrapolationEnd;

#[derive(Clone, Debug)]
pub enum TimelineEvent {
    Play,
    Pause,
    Stop,
    SetTime(f32),
    SetSpeed(f32),
    ToggleLoop,
    SelectClip(SourceClipId),
    ToggleTrack(BoneId),
    ExpandTrack(BoneId),
    CollapseTrack(BoneId),
    SelectKeyframe {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        modifier: SelectionModifier,
    },
    AddKeyframe {
        track: CurveTrackRef,
        property_type: PropertyType,
        time: f32,
        value: f32,
    },
    InsertKeyframeOnCurve {
        track: CurveTrackRef,
        property_type: PropertyType,
        time: f32,
    },
    DeleteSelectedKeyframes,
    MoveSelectedKeyframes {
        time_delta: f32,
    },
    SetKeyframeSelection {
        keyframes: Vec<SelectedKeyframe>,
        modifier: SelectionModifier,
    },
    DeleteKeyframe {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
    },
    MoveKeyframe {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        new_time: f32,
        new_value: f32,
    },
    SetKeyframeInterpolation {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        interpolation: InterpolationType,
    },
    SetKeyframeTangent {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        in_tangent: BezierHandle,
        out_tangent: BezierHandle,
    },
    SetTangentType {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        tangent_type: TangentType,
    },
    SetTangentWeightMode {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        weight_mode: TangentWeightMode,
    },
    SetTangentContinuity {
        track: CurveTrackRef,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        continuity: TangentContinuity,
    },
    SetSnapToFrame(bool),
    SetSnapToKey(bool),
    SetFrameRate(f32),
    CopyKeyframes,
    PasteKeyframes {
        paste_time: f32,
    },
    MirrorPaste {
        paste_time: f32,
    },
    CaptureBuffer,
    SwapBuffer,
    BoneSetKey,
    ZoomIn {
        max_zoom: f32,
    },
    ZoomOut {
        min_zoom: f32,
    },
    SetPreview(ClipPreview),
    SetCurveExtrapolation {
        track: CurveTrackRef,
        property_type: PropertyType,
        end: ExtrapolationEnd,
        mode: CurveExtrapolation,
    },
}

impl UiCommand for TimelineEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        let events = [*self];
        dispatch_timeline_events(&events, world, assets);
        dispatch_keyframe_clipboard_events(&events, world);
        dispatch_curve_buffer_events(&events, world);
    }
}

pub(super) fn dispatch_timeline_events(
    events: &[TimelineEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    apply_and_record_clip_edits(events, world);

    for event in events {
        match event {
            TimelineEvent::SelectClip(source_id) => retarget_schedules(world, *source_id),
            TimelineEvent::Play => clear_pose_overrides(world),
            _ => {}
        }
    }

    dispatch_bone_set_key_events(events, world, assets);
}

fn apply_and_record_clip_edits(events: &[TimelineEvent], world: &mut World) {
    let mut timeline_state = world.resource_mut::<TimelineState>();
    let mut clip_library = world.resource_mut::<ClipLibrary>();
    let snapshot = ClipSnapshot::capture(&timeline_state, &clip_library);
    let modified = apply_timeline_events(events, &mut timeline_state, &mut clip_library);
    drop(clip_library);
    drop(timeline_state);

    if !modified {
        return;
    }
    if let Some(snapshot) = snapshot {
        snapshot.record_mergeable(world, "timeline clip edit");
    }
    transition_to_baked_override_if_needed(world);
}

/// Scalar-domain entities play their own per-entity clip: timeline clip selection must never
/// repoint them (or reset their trim).
fn retarget_schedules(world: &mut World, source_id: SourceClipId) {
    let duration = {
        let lib = world.resource::<ClipLibrary>();
        let duration = lib.get(source_id).map(|c| c.duration).unwrap_or(1.0);
        log!(
            "[ClipSelect] source_id={}, asset_id={:?}, duration={:.3}",
            source_id,
            lib.get_asset_id_for_source(source_id),
            duration,
        );
        duration
    };

    for entity in world.component_entities::<ClipSchedule>() {
        if scalar_domain_for_entity(world, entity).is_some() {
            continue;
        }
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_switch_source(schedule, source_id, duration);
        }
    }
}

fn clear_pose_overrides(world: &mut World) {
    if let Some(mut overrides) = world.get_resource_mut::<BonePoseOverride>() {
        overrides.clear();
    }
}

/// Applies the transport, view and clip edits; true when the active clip changed.
pub(super) fn apply_timeline_events(
    events: &[TimelineEvent],
    timeline_state: &mut TimelineState,
    clip_library: &mut ClipLibrary,
) -> bool {
    for event in events {
        apply_view_event(event, timeline_state, clip_library);
    }

    let keyframes_modified = apply_keyframe_edits(events, timeline_state, clip_library);
    let tangents_modified = apply_tangent_edits(events, timeline_state, clip_library);
    keyframes_modified || tangents_modified
}

fn apply_view_event(
    event: &TimelineEvent,
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
) {
    match event {
        TimelineEvent::Play => timeline_state.playing = true,
        TimelineEvent::Pause => timeline_state.playing = false,
        TimelineEvent::Stop => {
            timeline_state.playing = false;
            timeline_state.current_time = 0.0;
        }
        TimelineEvent::SetTime(time) => {
            timeline_state.playing = false;
            timeline_state.set_time(*time);
        }
        TimelineEvent::SetSpeed(speed) => timeline_state.speed = *speed,
        TimelineEvent::ToggleLoop => timeline_state.looping = !timeline_state.looping,
        TimelineEvent::SelectClip(clip_id) => {
            timeline_select_clip(timeline_state, clip_library, *clip_id);
        }
        TimelineEvent::ToggleTrack(bone_id) => {
            timeline_toggle_track_expanded(timeline_state, *bone_id);
        }
        TimelineEvent::ExpandTrack(bone_id) => timeline_state.expand_track(*bone_id),
        TimelineEvent::CollapseTrack(bone_id) => timeline_state.collapse_track(*bone_id),
        TimelineEvent::SelectKeyframe {
            track,
            property_type,
            keyframe_id,
            modifier,
        } => {
            let selected = SelectedKeyframe::new(*track, *property_type, *keyframe_id);
            timeline_apply_selection(timeline_state, selected, *modifier);
        }
        TimelineEvent::SetKeyframeSelection {
            keyframes,
            modifier,
        } => {
            timeline_set_keyframe_selection(timeline_state, keyframes, *modifier);
        }
        TimelineEvent::SetSnapToFrame(enabled) => {
            timeline_state.snap_settings.snap_to_frame = *enabled;
        }
        TimelineEvent::SetSnapToKey(enabled) => {
            timeline_state.snap_settings.snap_to_key = *enabled;
        }
        TimelineEvent::SetFrameRate(rate) => {
            timeline_state.snap_settings.frame_rate = *rate;
        }
        TimelineEvent::ZoomIn { max_zoom } => timeline_zoom_in(timeline_state, *max_zoom),
        TimelineEvent::ZoomOut { min_zoom } => timeline_zoom_out(timeline_state, *min_zoom),
        TimelineEvent::SetPreview(preview) => timeline_state.preview = *preview,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{ClipInstance, EditableAnimationClip};
    use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
    use crate::ecs::systems::scalar_clip_systems::find_entity_clip_id;
    use crate::ecs::systems::scalar_clip_systems::test_support::spawn_probe_with_clip;

    #[test]
    fn timeline_select_clip_leaves_scalar_schedule_untouched() {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::new());
        world.insert_resource(TimelineState::new());
        let mut assets = AssetStorage::new();

        let probe = spawn_probe_with_clip(&mut world, &mut assets, "Probe");
        let probe_clip = find_entity_clip_id(&world, probe).expect("probe clip");

        let model = world.spawn();
        let model_clip = {
            let mut lib = world.resource_mut::<ClipLibrary>();
            let clip = EditableAnimationClip::new(0, "model_clip".to_string());
            clip_library_register_and_activate(&mut lib, &mut assets, clip)
        };
        let mut model_schedule = ClipSchedule::new();
        model_schedule
            .instances
            .push(ClipInstance::new(1, model_clip, 1.5));
        world.insert_component(model, model_schedule);

        world
            .get_component_mut::<ClipSchedule>(probe)
            .unwrap()
            .instances[0]
            .clip_out = 3.0;

        dispatch_timeline_events(
            &[TimelineEvent::SelectClip(probe_clip)],
            &mut world,
            &assets,
        );
        let inst = world
            .get_component::<ClipSchedule>(probe)
            .unwrap()
            .first_instance()
            .cloned()
            .unwrap();
        assert!(
            (inst.clip_out - 3.0).abs() < 1e-6,
            "scalar clip trim must survive double-click select"
        );

        dispatch_timeline_events(
            &[TimelineEvent::SelectClip(model_clip)],
            &mut world,
            &assets,
        );
        let inst = world
            .get_component::<ClipSchedule>(probe)
            .unwrap()
            .first_instance()
            .cloned()
            .unwrap();
        assert_eq!(inst.source_id, probe_clip);
        assert!((inst.clip_out - 3.0).abs() < 1e-6);
    }
}
