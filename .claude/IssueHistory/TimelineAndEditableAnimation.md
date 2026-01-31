# Timeline Window & Editable Animation Clip Implementation

## Summary
Implemented Timeline window and editable animation clip system for editing FBX/glTF imported animations.

## Implementation Date
2026-01-26

## Components Added

### Data Structures (`src/animation/editable/`)
- **keyframe.rs**: `EditableKeyframe`, `BezierHandle` - Keyframe with bezier tangent handles
- **curve.rs**: `PropertyCurve`, `PropertyType` - Per-property animation curves (TranslationX/Y/Z, RotationX/Y/Z/W, ScaleX/Y/Z)
- **track.rs**: `BoneTrack` - Contains all property curves for a single bone
- **clip.rs**: `EditableAnimationClip` - Editable version of AnimationClip with bidirectional conversion
- **manager.rs**: `EditableClipManager` - Manages multiple editable clips, save/load functionality

### UI Resources (`src/ecs/resource/`)
- **timeline_state.rs**: `TimelineState` - UI state for timeline (current clip, time, zoom, selected keyframes, expanded tracks)

### UI Events (`src/ecs/events/ui_events.rs`)
Added Timeline-related events:
- `TimelinePlay`, `TimelinePause`, `TimelineStop`
- `TimelineSetTime(f32)`, `TimelineSetSpeed(f32)`
- `TimelineToggleLoop`, `TimelineSelectClip(EditableClipId)`
- `TimelineToggleTrack(BoneId)`, `TimelineExpandTrack(BoneId)`, `TimelineCollapseTrack(BoneId)`
- `TimelineSelectKeyframe`, `TimelineAddKeyframe`, `TimelineDeleteSelectedKeyframes`
- `TimelineZoomIn`, `TimelineZoomOut`

### Timeline UI (`src/platform/ui/timeline_window.rs`)
- Transport controls (play/pause/stop, loop toggle, speed display)
- Clip selector dropdown
- Time ruler with zoom-dependent tick intervals
- Hierarchical track list with expand/collapse
- Keyframe visualization with color-coded properties
- Playhead indicator

### Systems (`src/ecs/systems/timeline_systems.rs`)
- `timeline_process_events()` - Event handler
- `timeline_update()` - Time advancement when playing
- `timeline_sync_from_playback()` - Sync from AnimationPlayback

### Serialization
- Added `ron = "0.8"` dependency
- All editable structures derive `Serialize` and `Deserialize`
- `.ranim` format (RON) for save/load via `EditableClipManager::save_to_file()` and `load_from_file()`

## Integration Points
- Resources registered in `src/app/init/instance.rs`
- Timeline window built in `src/platform/events.rs`
- Events processed in `process_timeline_events_inline()`

## File Format (.ranim)
```ron
(
    id: 1,
    name: "walk_cycle",
    duration: 1.0,
    tracks: {
        0: (
            bone_id: 0,
            bone_name: "Hips",
            translation_x: (
                id: 1,
                property_type: TranslationX,
                keyframes: [
                    (id: 1, time: 0.0, value: 0.0, ...),
                ],
                next_keyframe_id: 2,
            ),
            ...
        ),
    },
    source_path: None,
    next_curve_id: 11,
)
```

## Phase D: Blend System (2026-02-01)

### Summary
Implemented multi-clip blending: Override crossfade, Additive blending, ClipGroup mute/weight control.

### Changes

#### Publicized Helpers (`src/animation/data.rs`)
- `slerp()` and `normalize_quat()` changed from private to `pub` for use in pose blending

#### Pose Blend Functions (`src/ecs/systems/pose_blend_systems.rs`) [NEW]
- `blend_poses_override()` - Lerp/slerp between base and overlay poses
- `blend_poses_additive()` - Additive delta blending (translation add, rotation multiply)
- `apply_ease()` - EaseType curve evaluation (Linear, EaseIn, EaseOut, EaseInOut, Stepped)
- `compute_crossfade_factor()` - Calculates blend factor for overlapping override clips
- `compute_local_time()` - Converts global timeline time to clip-local time with speed/loop

#### Multi-Clip Blend Evaluation (`src/ecs/systems/animation_playback_systems.rs`)
- Replaced `group_by_pose()` + `apply_grouped_animations()` with `apply_blended_animations()`
- New `ActiveInstanceInfo` struct with per-instance blend data
- `build_active_instances()` collects all active clips at current time using `active_instances_at()`
- `evaluate_entity_blend()` evaluates full blend stack: first Override as base, subsequent Override via crossfade, Additive on top
- Uses `effective_instance_weight()` for group-aware weight calculation

#### ClipGroup (`src/animation/editable/clip_group.rs`) [NEW]
- `ClipGroup` struct: id, name, instance_ids, muted, weight

#### ClipSchedule Group Integration (`src/ecs/component/clip_schedule.rs`)
- Added `groups: Vec<ClipGroup>` field
- Methods: `create_group()`, `remove_group()`, `add_instance_to_group()`, `remove_instance_from_group()`, `find_group_for_instance()`, `effective_instance_weight()`
- `active_instances_at()` now respects group mute state

#### UIEvent Extensions (`src/ecs/events/ui_events.rs`)
- `ClipInstanceSetWeight`, `ClipInstanceSetBlendMode`
- `ClipGroupCreate`, `ClipGroupDelete`, `ClipGroupAddInstance`, `ClipGroupRemoveInstance`, `ClipGroupToggleMute`, `ClipGroupSetWeight`

#### Event Handlers (`src/ecs/systems/timeline_systems.rs`, `ui_event_systems.rs`)
- All new UIEvent variants handled in `process_clip_instance_events()`
- Pattern match in `process_ui_events_with_events_simple()` updated

#### Snapshot Blend Info (`src/platform/ui/clip_track_snapshot.rs`)
- `ClipInstanceSnapshot` now includes `weight`, `blend_mode`, `group_id`
- New `ClipGroupSnapshot` struct
- `ClipTrackEntry` now includes `groups`

#### Timeline UI (`src/platform/ui/timeline_window.rs`)
- Clip blocks display blend mode/weight: `"ClipName [O 0.75]"` or `"ClipName [A 1.0]"`
- Selected instance properties: Weight drag slider, BlendMode combo, Group assignment combo
- Group headers: Name, mute toggle, weight slider, delete button

### BlendMode Helper (`src/animation/editable/blend.rs`)
- Added `BlendMode::default_ease_in()` method

## Future Work
- Keyframe editing (drag to move, value editing)
- Curve editor with bezier handles
- Undo/redo system
