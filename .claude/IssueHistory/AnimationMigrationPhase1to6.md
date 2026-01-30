# Animation Migration Phase 1-6

## Summary
Migrated animation system from global AnimationPlayback-centric to entity-based Animator component evaluation. Renamed AnimationState to Animator, moved model_path to ModelState, added AnimationType enum, created ClipLibrary to unify AnimationRegistry + EditableClipManager, and introduced evaluate_animators for entity-based animation evaluation.

## Date
2026-01-31

## Phase 1: AnimationState -> Animator Rename
- Renamed `AnimationState` struct to `Animator` in `world.rs`
- Updated `with_animation_state()` builder to `with_animator()`
- Changed all references in `model_loader.rs`, `ecs_world_systems.rs`

## Phase 2: Animator <-> AnimationPlayback Bridge
- Created sync logic in `frame_runner.rs` to bridge Animator component and AnimationPlayback resource
- Playback -> Animator sync runs after timeline_update so Animator stays in sync

## Phase 3: model_path Moved to ModelState
- Added `AnimationType` enum (None, Skeletal, Node) to `graphics.rs`
- Extended `ModelState` with `model_path` and `animation_type` fields
- Removed `model_path`, `current_index`, `with_model_path()` from `AnimationPlayback`
- `playback_prepare_animations` now uses `AnimationType` instead of path string comparison
- Updated `events.rs`, `render.rs`, `scene_model.rs`, `scene_io.rs`, `instance.rs`

## Phase 4: TimelineState target_entity
- Added `target_entity: Option<Entity>` to `TimelineState`
- Synced from `HierarchyState.selected_entity` each frame in `frame_runner.rs`

## Phase 5: Entity-Based Evaluation
- Added `evaluate_animators()` function that reads animation time from Animator component instead of AnimationPlayback
- Changed `apply_to_skeleton()` signature to take individual params (clip_id, time, looping) instead of AnimationPlayback reference
- `animation_phase.rs` now calls `evaluate_animators` instead of `playback_prepare_animations`
- Removed `animator_sync_systems.rs` (bridge no longer needed for animation evaluation)
- Kept Playback->Animator sync in timeline_phase for UI consistency

## Phase 6: ClipLibrary Integration
- Created `ClipLibrary` in `ecs/resource/clip_library.rs` combining:
  - `AnimationRegistry` (animation system + morph animation)
  - `EditableClipManager` (editable clips, dirty tracking, sync)
- Added `sync_dirty_clips()` method that internally syncs dirty editable clips to animation.clips
- Removed `AnimationRegistry` from `graphics.rs`
- Updated all references across ~15 files
- `EditableClipManager` definition kept in `animation/editable/manager.rs` but no longer referenced externally

## Key Architectural Changes
- Animation evaluation now reads from entity Animator components, not global AnimationPlayback
- Model type determined by AnimationType enum, not path string comparison
- Single ClipLibrary resource replaces AnimationRegistry + EditableClipManager
