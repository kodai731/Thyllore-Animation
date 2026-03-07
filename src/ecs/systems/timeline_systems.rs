use std::collections::HashMap;

use crate::animation::editable::{
    curve_recalculate_auto_tangent_at, curve_remove_keyframe, initialize_weighted_handle_lengths,
    PropertyType, SourceClipId, TangentWeightMode,
};
use crate::animation::{BoneId, BoneLocalPose};
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::world::World;

pub fn timeline_process_events(
    events: &[UIEvent],
    timeline_state: &mut TimelineState,
    clip_library: &mut ClipLibrary,
) -> bool {
    let mut clip_modified = false;

    for event in events {
        match event {
            UIEvent::TimelinePlay => {
                timeline_state.playing = true;
            }

            UIEvent::TimelinePause => {
                timeline_state.playing = false;
            }

            UIEvent::TimelineStop => {
                timeline_state.playing = false;
                timeline_state.current_time = 0.0;
            }

            UIEvent::TimelineSetTime(time) => {
                timeline_state.playing = false;
                timeline_state.set_time(*time);
            }

            UIEvent::TimelineSetSpeed(speed) => {
                timeline_state.speed = *speed;
            }

            UIEvent::TimelineToggleLoop => {
                timeline_state.looping = !timeline_state.looping;
            }

            UIEvent::TimelineSelectClip(clip_id) => {
                timeline_select_clip(timeline_state, clip_library, *clip_id);
            }

            UIEvent::TimelineToggleTrack(bone_id) => {
                timeline_state.toggle_track_expanded(*bone_id);
            }

            UIEvent::TimelineExpandTrack(bone_id) => {
                timeline_state.expand_track(*bone_id);
            }

            UIEvent::TimelineCollapseTrack(bone_id) => {
                timeline_state.collapse_track(*bone_id);
            }

            UIEvent::TimelineSelectKeyframe {
                bone_id,
                property_type,
                keyframe_id,
                modifier,
            } => {
                use crate::ecs::resource::SelectedKeyframe;
                let selected = SelectedKeyframe::new(*bone_id, *property_type, *keyframe_id);
                timeline_state.apply_selection(selected, *modifier);
            }

            UIEvent::TimelineAddKeyframe {
                bone_id,
                property_type,
                time,
                value,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        clip.add_keyframe(*bone_id, *property_type, *time, *value);
                        clip.recalculate_duration();
                        clip_modified = true;
                    }
                }
            }

            UIEvent::TimelineMoveSelectedKeyframes { time_delta } => {
                crate::log!(
                    "[Timeline] MoveSelectedKeyframes: selected={}, delta={:.4}",
                    timeline_state.selected_keyframes.len(),
                    time_delta
                );
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        for sel in &timeline_state.selected_keyframes {
                            if let Some(track) = clip.tracks.get_mut(&sel.bone_id) {
                                let curve = track.get_curve_mut(sel.property_type);
                                if let Some(kf) =
                                    curve.keyframes.iter_mut().find(|k| k.id == sel.keyframe_id)
                                {
                                    crate::log!(
                                        "[Timeline]   moved: bone={} {:?} kf={} time {:.3} -> {:.3}",
                                        sel.bone_id, sel.property_type, sel.keyframe_id,
                                        kf.time, (kf.time + time_delta).max(0.0)
                                    );
                                    kf.time = (kf.time + time_delta).max(0.0);
                                }
                            }
                        }
                        clip.recalculate_duration();
                        clip_modified = true;
                    }
                }
            }

            UIEvent::TimelineSetKeyframeSelection {
                keyframes,
                modifier,
            } => {
                use crate::ecs::resource::SelectionModifier;
                match modifier {
                    SelectionModifier::Replace => {
                        timeline_state.selected_keyframes.clear();
                        for kf in keyframes {
                            timeline_state.selected_keyframes.insert(kf.clone());
                        }
                    }
                    SelectionModifier::Add => {
                        for kf in keyframes {
                            timeline_state.selected_keyframes.insert(kf.clone());
                        }
                    }
                    SelectionModifier::Toggle => {
                        for kf in keyframes {
                            if timeline_state.selected_keyframes.contains(kf) {
                                timeline_state.selected_keyframes.remove(kf);
                            } else {
                                timeline_state.selected_keyframes.insert(kf.clone());
                            }
                        }
                    }
                }
            }

            UIEvent::TimelineDeleteSelectedKeyframes => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    let selected: Vec<_> =
                        timeline_state.selected_keyframes.iter().cloned().collect();
                    if !selected.is_empty() {
                        if let Some(clip) = clip_library.get_mut(clip_id) {
                            for sel in &selected {
                                if let Some(track) = clip.tracks.get_mut(&sel.bone_id) {
                                    curve_remove_keyframe(
                                        track.get_curve_mut(sel.property_type),
                                        sel.keyframe_id,
                                    );
                                }
                            }
                            clip.recalculate_duration();
                            clip_modified = true;
                        }
                    }
                    timeline_state.clear_selection();
                }
            }

            UIEvent::TimelineMoveKeyframe {
                bone_id,
                property_type,
                keyframe_id,
                new_time,
                new_value,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            track.move_keyframe(
                                *property_type,
                                *keyframe_id,
                                *new_time,
                                *new_value,
                            );
                        }
                        clip.recalculate_duration();
                        clip_modified = true;
                    }
                }
            }

            UIEvent::TimelineDeleteKeyframe {
                bone_id,
                property_type,
                keyframe_id,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            curve_remove_keyframe(
                                track.get_curve_mut(*property_type),
                                *keyframe_id,
                            );
                        }
                        clip.recalculate_duration();
                        clip_modified = true;
                    }
                }
            }

            UIEvent::TimelineZoomIn => {
                timeline_state.zoom_in();
            }

            UIEvent::TimelineZoomOut => {
                timeline_state.zoom_out();
            }

            UIEvent::TimelineSetSnapToFrame(enabled) => {
                timeline_state.snap_settings.snap_to_frame = *enabled;
            }

            UIEvent::TimelineSetSnapToKey(enabled) => {
                timeline_state.snap_settings.snap_to_key = *enabled;
            }

            UIEvent::TimelineSetFrameRate(rate) => {
                timeline_state.snap_settings.frame_rate = *rate;
            }

            UIEvent::TimelineSetKeyframeInterpolation {
                bone_id,
                property_type,
                keyframe_id,
                interpolation,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            track
                                .get_curve_mut(*property_type)
                                .set_keyframe_interpolation(*keyframe_id, *interpolation);
                            clip_modified = true;
                        }
                    }
                }
            }

            UIEvent::TimelineSetKeyframeTangent {
                bone_id,
                property_type,
                keyframe_id,
                in_tangent,
                out_tangent,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            track.get_curve_mut(*property_type).set_keyframe_tangents(
                                *keyframe_id,
                                in_tangent.clone(),
                                out_tangent.clone(),
                            );
                            clip_modified = true;
                        }
                    }
                }
            }

            UIEvent::TimelineAutoTangent {
                bone_id,
                property_type,
                keyframe_id,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            curve_recalculate_auto_tangent_at(
                                track.get_curve_mut(*property_type),
                                *keyframe_id,
                            );
                            clip_modified = true;
                        }
                    }
                }
            }

            UIEvent::TimelineSetTangentWeightMode {
                bone_id,
                property_type,
                keyframe_id,
                weight_mode,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            let curve = track.get_curve_mut(*property_type);
                            curve.set_keyframe_weight_mode(*keyframe_id, *weight_mode);
                            if *weight_mode == TangentWeightMode::Weighted {
                                if let Some(idx) =
                                    curve.keyframes.iter().position(|k| k.id == *keyframe_id)
                                {
                                    let dt = if curve.keyframes.len() >= 2 {
                                        let first = curve.keyframes.first().unwrap().time;
                                        let last = curve.keyframes.last().unwrap().time;
                                        ((last - first)
                                            / (curve.keyframes.len() as f32 - 1.0).max(1.0))
                                        .max(0.1)
                                    } else {
                                        1.0
                                    };
                                    initialize_weighted_handle_lengths(
                                        &mut curve.keyframes[idx],
                                        dt,
                                    );
                                }
                            }
                            clip_modified = true;
                        }
                    }
                }
            }

            _ => {}
        }
    }

    clip_modified
}

fn timeline_select_clip(
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
    clip_id: SourceClipId,
) {
    if let Some(clip) = clip_library.get(clip_id) {
        timeline_state.current_clip_id = Some(clip_id);
        timeline_state.current_time = 0.0;
        timeline_state.selected_keyframes.clear();
        timeline_state.expanded_tracks.clear();

        if let Some((&first_bone_id, _)) = clip.tracks.iter().next() {
            timeline_state.expand_track(first_bone_id);
        }

        crate::log!(
            "Timeline: Selected clip '{}' (id={}, duration={:.2}s, tracks={})",
            clip.name,
            clip_id,
            clip.duration,
            clip.track_count()
        );
    }
}

pub fn timeline_update(
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
    delta_time: f32,
) {
    if !timeline_state.playing {
        return;
    }

    let duration = timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
        .map(|c| c.duration)
        .unwrap_or(0.0);

    if duration <= 0.0 {
        return;
    }

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

pub fn process_clip_instance_events(events: &[UIEvent], world: &mut World) {
    let mut deselect_after: Option<(
        crate::ecs::world::Entity,
        crate::animation::editable::ClipInstanceId,
    )> = None;

    for event in events {
        match event {
            UIEvent::ClipInstanceSelect {
                entity,
                instance_id,
            } => {
                let source_id = world
                    .get_component::<ClipSchedule>(*entity)
                    .and_then(|schedule| {
                        schedule
                            .instances
                            .iter()
                            .find(|i| i.instance_id == *instance_id)
                            .map(|i| i.source_id)
                    });

                let mut ts = world.resource_mut::<TimelineState>();
                ts.selected_clip_instance = Some((*entity, *instance_id));

                if let Some(source_id) = source_id {
                    if ts.current_clip_id != Some(source_id) {
                        let clip_library = world.resource::<ClipLibrary>();
                        if let Some(clip) = clip_library.get(source_id) {
                            ts.current_clip_id = Some(source_id);
                            ts.current_time = 0.0;
                            ts.selected_keyframes.clear();
                            ts.expanded_tracks.clear();

                            if let Some((&first_bone_id, _)) = clip.tracks.iter().next() {
                                ts.expand_track(first_bone_id);
                            }
                        }
                    }
                }
            }

            UIEvent::ClipInstanceDeselect => {
                let mut ts = world.resource_mut::<TimelineState>();
                ts.selected_clip_instance = None;
            }

            UIEvent::ClipInstanceMove {
                entity,
                instance_id,
                new_start_time,
            } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.start_time = *new_start_time;
                });
            }

            UIEvent::ClipInstanceTrimStart {
                entity,
                instance_id,
                new_clip_in,
            } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.clip_in = new_clip_in.max(0.0);
                });
            }

            UIEvent::ClipInstanceTrimEnd {
                entity,
                instance_id,
                new_clip_out,
            } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.clip_out = new_clip_out.max(0.0);
                });
            }

            UIEvent::ClipInstanceToggleMute {
                entity,
                instance_id,
            } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.muted = !inst.muted;
                });
            }

            UIEvent::ClipInstanceDelete {
                entity,
                instance_id,
            } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    super::clip_schedule_systems::clip_schedule_remove_instance(
                        schedule,
                        *instance_id,
                    );
                }
                deselect_after = Some((*entity, *instance_id));
            }

            UIEvent::ClipInstanceSetWeight {
                entity,
                instance_id,
                weight,
            } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.weight = weight.clamp(0.0, 1.0);
                });
            }

            UIEvent::ClipInstanceSetBlendMode {
                entity,
                instance_id,
                blend_mode,
            } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.blend_mode = *blend_mode;
                });
            }

            UIEvent::ClipGroupCreate { entity, name } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    super::clip_schedule_systems::clip_schedule_create_group(
                        schedule,
                        name.clone(),
                    );
                }
            }

            UIEvent::ClipGroupDelete { entity, group_id } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    super::clip_schedule_systems::clip_schedule_remove_group(schedule, *group_id);
                }
            }

            UIEvent::ClipGroupAddInstance {
                entity,
                group_id,
                instance_id,
            } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    super::clip_schedule_systems::clip_schedule_add_to_group(
                        schedule,
                        *group_id,
                        *instance_id,
                    );
                }
            }

            UIEvent::ClipGroupRemoveInstance {
                entity,
                group_id,
                instance_id,
            } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    super::clip_schedule_systems::clip_schedule_remove_from_group(
                        schedule,
                        *group_id,
                        *instance_id,
                    );
                }
            }

            UIEvent::ClipGroupToggleMute { entity, group_id } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    if let Some(group) = schedule.groups.iter_mut().find(|g| g.id == *group_id) {
                        group.muted = !group.muted;
                    }
                }
            }

            UIEvent::ClipGroupSetWeight {
                entity,
                group_id,
                weight,
            } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    if let Some(group) = schedule.groups.iter_mut().find(|g| g.id == *group_id) {
                        group.weight = weight.clamp(0.0, 1.0);
                    }
                }
            }

            _ => {}
        }
    }

    if let Some((entity, instance_id)) = deselect_after {
        let mut ts = world.resource_mut::<TimelineState>();
        if let Some((sel_entity, sel_id)) = ts.selected_clip_instance {
            if sel_entity == entity && sel_id == instance_id {
                ts.selected_clip_instance = None;
            }
        }
    }
}

fn modify_clip_instance(
    world: &mut World,
    entity: crate::ecs::world::Entity,
    instance_id: crate::animation::editable::ClipInstanceId,
    f: impl FnOnce(&mut crate::animation::editable::ClipInstance),
) {
    if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
        if let Some(inst) = schedule
            .instances
            .iter_mut()
            .find(|i| i.instance_id == instance_id)
        {
            f(inst);
        }
    }
}

pub fn process_bone_set_key(
    overrides: &HashMap<BoneId, BoneLocalPose>,
    clip_library: &mut ClipLibrary,
    timeline_state: &TimelineState,
    skeleton: &crate::animation::Skeleton,
) -> bool {
    let Some(clip_id) = timeline_state.current_clip_id else {
        return false;
    };
    let Some(clip) = clip_library.get_mut(clip_id) else {
        return false;
    };

    if overrides.is_empty() {
        return false;
    }

    let time = timeline_state.current_time;

    for (&bone_id, local_pose) in overrides {
        let bone_name = skeleton
            .get_bone(bone_id)
            .map(|b| b.name.clone())
            .unwrap_or_else(|| format!("bone_{}", bone_id));

        if !clip.tracks.contains_key(&bone_id) {
            clip.add_track(bone_id, bone_name.clone());
        }

        let euler = crate::animation::editable::quaternion_to_euler_degrees(&local_pose.rotation);

        let t = &local_pose.translation;
        let s = &local_pose.scale;
        clip.add_keyframe(bone_id, PropertyType::TranslationX, time, t.x);
        clip.add_keyframe(bone_id, PropertyType::TranslationY, time, t.y);
        clip.add_keyframe(bone_id, PropertyType::TranslationZ, time, t.z);
        clip.add_keyframe(bone_id, PropertyType::RotationX, time, euler.x);
        clip.add_keyframe(bone_id, PropertyType::RotationY, time, euler.y);
        clip.add_keyframe(bone_id, PropertyType::RotationZ, time, euler.z);
        clip.add_keyframe(bone_id, PropertyType::ScaleX, time, s.x);
        clip.add_keyframe(bone_id, PropertyType::ScaleY, time, s.y);
        clip.add_keyframe(bone_id, PropertyType::ScaleZ, time, s.z);
    }

    clip.recalculate_duration();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{
        EditableAnimationClip, PropertyType, SourceClip, SourceClipId,
    };
    use crate::ecs::resource::{SelectedKeyframe, SelectionModifier};

    fn setup_test_clip() -> (TimelineState, ClipLibrary) {
        let clip_id: SourceClipId = 1;
        let mut clip = EditableAnimationClip::new(clip_id, "test".to_string());
        let bone_id = 0;
        clip.add_track(bone_id, "bone0".to_string());

        let kf1_id = clip
            .add_keyframe(bone_id, PropertyType::TranslationX, 0.5, 1.0)
            .unwrap();
        let kf2_id = clip
            .add_keyframe(bone_id, PropertyType::TranslationX, 1.0, 2.0)
            .unwrap();
        let kf3_id = clip
            .add_keyframe(bone_id, PropertyType::TranslationY, 0.8, 3.0)
            .unwrap();
        clip.recalculate_duration();

        let mut library = ClipLibrary::new();
        library
            .source_clips
            .insert(clip_id, SourceClip::new(clip_id, clip));

        let mut state = TimelineState::new();
        state.current_clip_id = Some(clip_id);

        state.selected_keyframes.insert(SelectedKeyframe::new(
            bone_id,
            PropertyType::TranslationX,
            kf1_id,
        ));
        state.selected_keyframes.insert(SelectedKeyframe::new(
            bone_id,
            PropertyType::TranslationX,
            kf2_id,
        ));
        state.selected_keyframes.insert(SelectedKeyframe::new(
            bone_id,
            PropertyType::TranslationY,
            kf3_id,
        ));

        (state, library)
    }

    #[test]
    fn move_selected_keyframes_shifts_time() {
        let (mut state, mut library) = setup_test_clip();
        let events = vec![UIEvent::TimelineMoveSelectedKeyframes { time_delta: 0.25 }];

        let modified = timeline_process_events(&events, &mut state, &mut library);
        assert!(modified);

        let clip = library.get(1).unwrap();
        let track = clip.tracks.get(&0).unwrap();
        let tx_curve = track.get_curve(PropertyType::TranslationX);

        let times: Vec<f32> = tx_curve.keyframes.iter().map(|k| k.time).collect();
        assert!(
            (times[0] - 0.75).abs() < 0.01,
            "Expected 0.75, got {}",
            times[0]
        );
        assert!(
            (times[1] - 1.25).abs() < 0.01,
            "Expected 1.25, got {}",
            times[1]
        );

        let ty_curve = track.get_curve(PropertyType::TranslationY);
        let ty_time = ty_curve.keyframes[0].time;
        assert!(
            (ty_time - 1.05).abs() < 0.01,
            "Expected 1.05, got {}",
            ty_time
        );
    }

    #[test]
    fn move_selected_keyframes_clamps_at_zero() {
        let (mut state, mut library) = setup_test_clip();
        let events = vec![UIEvent::TimelineMoveSelectedKeyframes { time_delta: -2.0 }];

        timeline_process_events(&events, &mut state, &mut library);

        let clip = library.get(1).unwrap();
        let track = clip.tracks.get(&0).unwrap();
        let tx_curve = track.get_curve(PropertyType::TranslationX);

        for kf in &tx_curve.keyframes {
            assert!(kf.time >= 0.0, "Time should be >= 0, got {}", kf.time);
        }
    }

    #[test]
    fn set_keyframe_selection_replace() {
        let (mut state, mut library) = setup_test_clip();

        let new_sel = vec![SelectedKeyframe::new(0, PropertyType::ScaleX, 999)];
        let events = vec![UIEvent::TimelineSetKeyframeSelection {
            keyframes: new_sel,
            modifier: SelectionModifier::Replace,
        }];

        timeline_process_events(&events, &mut state, &mut library);

        assert_eq!(state.selected_keyframes.len(), 1);
        assert!(state.selected_keyframes.contains(&SelectedKeyframe::new(
            0,
            PropertyType::ScaleX,
            999
        )));
    }

    #[test]
    fn set_keyframe_selection_add() {
        let (mut state, mut library) = setup_test_clip();
        let original_count = state.selected_keyframes.len();

        let new_sel = vec![SelectedKeyframe::new(0, PropertyType::ScaleX, 999)];
        let events = vec![UIEvent::TimelineSetKeyframeSelection {
            keyframes: new_sel,
            modifier: SelectionModifier::Add,
        }];

        timeline_process_events(&events, &mut state, &mut library);

        assert_eq!(state.selected_keyframes.len(), original_count + 1);
    }

    #[test]
    fn set_keyframe_selection_toggle() {
        let (mut state, mut library) = setup_test_clip();

        // Toggle off an existing keyframe, toggle on a new one
        let existing = state.selected_keyframes.iter().next().unwrap().clone();
        let new_kf = SelectedKeyframe::new(0, PropertyType::ScaleX, 999);
        let events = vec![UIEvent::TimelineSetKeyframeSelection {
            keyframes: vec![existing.clone(), new_kf.clone()],
            modifier: SelectionModifier::Toggle,
        }];

        let before_count = state.selected_keyframes.len();
        timeline_process_events(&events, &mut state, &mut library);

        // One removed, one added → count stays same
        assert_eq!(state.selected_keyframes.len(), before_count);
        assert!(!state.selected_keyframes.contains(&existing));
        assert!(state.selected_keyframes.contains(&new_kf));
    }
}
