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

## Future Work
- Keyframe editing (drag to move, value editing)
- Curve editor with bezier handles
- Undo/redo system
- Per-entity Animator component
