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

## Phase E: Tangent / Interpolation System (2026-02-01)

### Summary
Added per-keyframe interpolation types (Linear/Bezier/Stepped) and tangent handle editing to the Curve Editor.

### Changes

#### InterpolationType (`src/animation/editable/keyframe.rs`)
- Added `InterpolationType` enum: `Linear` (default), `Bezier`, `Stepped`
- Added `interpolation` field to `EditableKeyframe` with `#[serde(default)]` for backward compatibility

#### Bezier Sampling & Tangent Calculation (`src/animation/editable/tangent.rs`) [NEW]
- `sample_bezier()` - Cubic bezier sampling with Newton-Raphson time-axis inversion (8 iterations)
- `apply_auto_tangent()` - Catmull-Rom style automatic tangent calculation
- `apply_flat_tangent()` - Horizontal tangent (value_offset = 0)
- `apply_linear_tangent()` - Straight-line tangent to neighboring keyframes
- `apply_auto_tangents_to_all()` - Batch auto tangent for all keyframes
- 5 unit tests covering bezier sampling, auto/flat tangent generation

#### PropertyCurve Bezier Support (`src/animation/editable/curve.rs`)
- `sample()` now dispatches per-keyframe: Stepped returns k0.value, Linear interpolates, Bezier calls `sample_bezier()`
- Added `set_keyframe_interpolation()`, `set_keyframe_tangents()`, `recalculate_auto_tangents()`, `recalculate_auto_tangent_at()`, `has_bezier_keyframes()`

#### UIEvents (`src/ecs/events/ui_events.rs`)
- `TimelineSetKeyframeInterpolation` - Change keyframe interpolation type
- `TimelineSetKeyframeTangent` - Set in/out tangent handles directly
- `TimelineAutoTangent` - Auto-calculate tangent for a keyframe

#### Event Handlers (`src/ecs/systems/timeline_systems.rs`, `ui_event_systems.rs`)
- Pattern match for all 3 new events in both systems

#### Curve Editor UI (`src/platform/ui/curve_editor_window.rs`)
- `TangentHandleType` enum and `DraggingTangent` struct for tangent drag state
- `draw_tangent_handles()` - Renders in/out handle lines and squares for Bezier keyframes
- `draw_curve_with_keyframes()` rewritten: per-segment sampling (Stepped: 2-step staircase, Linear: 2 samples, Bezier: 20 samples)
- `find_tangent_handle_at_position()` - Hit testing for tangent handle squares
- `handle_curve_area_click()` prioritizes tangent handle hits over keyframe hits
- `handle_mouse_release()` emits `TimelineSetKeyframeTangent` on tangent drag end
- Right-click context menu: Interpolation type selection (Linear/Bezier/Stepped), Tangent presets (Auto/Flat/Reset)

#### Bake Support (`src/animation/editable/clip.rs`)
- `collect_bake_times()` inserts 10 intermediate samples per Bezier segment for dense linear bake
- `to_animation_clip()` uses `collect_bake_times()` instead of `collect_unique_times()`

---

## Phase F: Dope Sheet + Detailed Editing (2026-02-01)

### Summary
Added Dope Sheet view mode, keyframe clipboard (copy/paste/mirror paste), snap settings, and buffer curve system.

### New Files
- `src/animation/editable/snap.rs` - Snap-to-frame/key calculations
- `src/animation/editable/mirror.rs` - Mirror mapping for L/R bone symmetry, mirror paste
- `src/ecs/resource/keyframe_copy_buffer.rs` - Keyframe copy buffer for clipboard operations
- `src/ecs/resource/curve_editor_buffer.rs` - Curve snapshot buffer for capture/swap
- `src/ecs/systems/keyframe_clipboard_systems.rs` - Copy/paste/mirror paste system functions
- `src/platform/ui/dope_sheet.rs` - Dope Sheet rendering (summary row, collapsed/expanded bone rows, diamond keyframe markers)

### Modified Files
- `timeline_state.rs` - Added TimelineViewMode, SnapSettings, SelectionModifier, apply_selection()
- `ui_events.rs` - Added modifier field to TimelineSelectKeyframe, new events (Copy/Paste/Mirror/Snap/ViewMode/Buffer)
- `timeline_systems.rs` - Implemented Add/Delete keyframe handlers, ViewMode/Snap handlers
- `timeline_window.rs` - Tab switching (Dope Sheet / Graph Editor), snap controls, keyboard shortcuts
- `curve_editor_window.rs` - Buffer curve overlay, Capture/Swap buttons
- `events.rs` - Clipboard/buffer event processing, resource initialization

### Key Design Decisions
- Dope Sheet in separate file to avoid timeline_window.rs bloat
- SelectionModifier determined from Shift/Ctrl keys at click time (not persisted state)
- MirrorMapping built on-demand from bone names (L_/R_, Left/Right, .L/.R patterns)
- Copy buffer stores relative times (base_time subtracted)

### Keyboard Shortcuts (Timeline window focused)
- Ctrl+C: Copy selected keyframes
- Ctrl+V: Paste at current time
- Ctrl+Shift+V: Mirror paste at current time
- Delete: Delete selected keyframes
- Tab: Toggle Dope Sheet / Graph Editor

## Phase G: Undo/Redo + Clip Browser (2026-02-01)

### Summary
Implemented Undo/Redo system (Snapshot pattern) and Clip Browser window with D&D clip binding to timeline tracks.

### New Files
- `src/ecs/resource/edit_history.rs` - EditHistory, EditCommand, EditEntry, EditCommandAfter (Snapshot-based undo/redo)
- `src/ecs/resource/clip_browser_state.rs` - ClipBrowserState (filter, selection)
- `src/ecs/systems/edit_history_systems.rs` - apply_undo(), apply_redo() system functions
- `src/platform/ui/clip_browser_window.rs` - Clip Browser UI (list, filter, D&D source, toolbar)

### Modified Files
- `ui_events.rs` - Added Undo, Redo, ClipInstanceAdd, ClipBrowserCreateEmpty/Duplicate/Delete events
- `ui_event_systems.rs` - Pattern match for new event variants
- `timeline_systems.rs` - timeline_process_events() now returns bool (clip modified flag)
- `timeline_window.rs` - Ctrl+Z/Y shortcuts, D&D target on clip tracks
- `hierarchy_window.rs` - Height reduced to 60% of main area (Clip Browser uses remaining)
- `events.rs` - process_edit_history_events_inline(), process_clip_browser_events_inline(), snapshot recording in timeline/clip instance processing
- `instance.rs` / `model_loader.rs` - EditHistory and ClipBrowserState resource initialization
- `resource/mod.rs` / `systems/mod.rs` / `ui/mod.rs` - Module registrations

### Key Design Decisions
- **Snapshot pattern** over Command pattern: EditableAnimationClip and ClipSchedule are Clone-able, making before/after snapshots trivial
- **1 UIEvent = 1 Undo unit**: No grouping (future extension)
- **RefCell borrow avoidance**: Raw pointer trick for simultaneous EditHistory + ClipLibrary + World mutation
- **Clip Browser placement**: Below Hierarchy window (250px width, hierarchy=60% height, browser=40%)
- **D&D protocol**: imgui drag_drop_source/target with "CLIP_SOURCE" payload containing SourceClipId (u64, Copy)
- **Reference counting**: ClipBrowserDelete only succeeds if no ClipInstance references the source clip

### Keyboard Shortcuts
- Ctrl+Z: Undo
- Ctrl+Y: Redo

### Tests
- 4 unit tests in edit_history.rs: push/undo, push/redo, redo_cleared_on_new_edit, max_history_limit

## Future Work
- Tangent lock/break modes
- Graph Editor zoom-to-fit for selected curves
- Keyframe drag in Dope Sheet
- Undo grouping for batch operations
