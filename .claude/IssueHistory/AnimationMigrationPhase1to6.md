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

---

## Skeleton Ownership Refactoring + ECS Compliance (2026-01-31)

### Summary
Separated Skeleton from AnimationSystem, made AssetStorage the single source of truth for Skeleton data, introduced SkeletonPose for per-frame pose calculation (eliminating bone.local_transform dual responsibility), and moved all animation calculation methods from data.rs to ECS system functions.

### Problem
1. AnimationSystem owned both skeletons and clips. Skeleton is mesh structure data, not runtime state.
2. bone.local_transform served dual purpose: rest pose at load time, but overwritten each frame by sample_with_loop, losing rest pose.
3. AnimationClip::sample, Skeleton::compute_global_transforms, SkinData::apply_skinning were methods on data structs, violating ECS data-behavior separation.

### Solution

**SkeletonPose + skeleton_pose_systems:**
- Created `animation/pose.rs` with `BoneLocalPose` and `SkeletonPose` (data-only)
- Created `ecs/systems/skeleton_pose_systems.rs` with all calculation logic:
  - `create_pose_from_rest()`, `sample_clip_to_pose()`, `compute_pose_global_transforms()`, `compute_rest_global_transforms()`, `apply_skinning()`
- Skeleton's bone.local_transform is never mutated at runtime

**AssetStorage as single source of truth:**
- Added `get_skeleton_by_skeleton_id(SkeletonId) -> Option<&Skeleton>` to AssetStorage
- Skeletons registered in AssetStorage at load time (setup_animation_system)
- evaluate_animators and all animation paths now read Skeleton from AssetStorage, not AnimationSystem

**Removed from AnimationSystem:**
- `apply_to_skeleton()` method deleted
- Removed from AnimationClip: `sample()`, `sample_with_loop()`
- Removed from Skeleton: `compute_global_transforms()`
- Removed from SkinData: `apply_skinning()`
- AnimationSystem.skeletons field still exists for loader compatibility but is no longer the authoritative source

**Changed functions:**
- `evaluate_animators`: now takes `&AssetStorage`, uses pose-based pipeline
- `playback_prepare_animations`: now takes `&AssetStorage`, `&ClipLibrary` (no longer mut)
- `prepare_skinned_vertices`: takes `&[Matrix4]` + `&Skeleton` instead of `&AnimationSystem`
- `prepare_node_animation`: takes `&Skeleton` + `&SkeletonPose` instead of `&AnimationSystem`
- `compute_node_global_transforms`: takes `&Skeleton` + `&SkeletonPose` instead of `&AnimationSystem`
- `apply_skinning_to_mesh`: takes `&[Matrix4]` + `&Skeleton` instead of `&AnimationSystem`
- `skeleton_animation_system`: takes `&AssetStorage` (immutable) instead of `&mut AssetStorage`
- gltf loader: `clip.sample(0.0, skeleton)` replaced with `initialize_skeleton_from_clip()` helper

---

## Phase A: Source/Instance + ClipSchedule Introduction (2026-01-31)

### Summary
Introduced SourceClip/ClipInstance/ClipSchedule architecture to replace direct current_clip_id references in Animator and AnimationPlayback. Renamed EditableClipId to SourceClipId across the codebase.

### New Data Structures
- `BlendMode` (Override, Additive) and `EaseType` (Linear, EaseIn, EaseOut, EaseInOut, Stepped) in `animation/editable/blend.rs`
- `SourceClip` wraps EditableAnimationClip with id and ref_count in `animation/editable/source_clip.rs`
- `ClipInstance` represents a placed clip on a timeline with timing, blend, and ease params in `animation/editable/clip_instance.rs`
- `ClipSchedule` ECS component holds Vec<ClipInstance> per entity in `ecs/component/clip_schedule.rs`

### ClipLibrary Refactoring
- Internal storage changed from `HashMap<EditableClipId, EditableAnimationClip>` to `HashMap<SourceClipId, SourceClip>`
- Fields renamed: editable_clips -> source_clips, dirty_clips -> dirty_sources, next_editable_id -> next_source_id, editable_to_anim_id -> source_to_anim_id
- New methods: `get_source()`, `get_source_mut()`, `get_anim_clip_id_for_source()`, `find_source_id_for_anim_clip()`
- Public API `get()` / `get_mut()` still return `&EditableAnimationClip` via delegation

### EditableClipId -> SourceClipId Rename
- Replaced across 9 files: keyframe.rs, clip.rs, manager.rs, clip_library.rs, timeline_state.rs, ui_events.rs, timeline_systems.rs, scene_io.rs, instance.rs
- EditableClipId type alias removed after all references migrated

### current_clip_id Removal
- **Animator.current_clip_id** removed (7 references across 5 files)
- **AnimationPlayback.current_clip_id** removed (5 references across 4 files)
- Both replaced by ClipSchedule-based resolution via `resolve_active_clip()` and `resolve_clip_id_for_entity()` helpers
- evaluate_animators, animation_time_system, skeleton_animation_system all now resolve clip from ClipSchedule
- sync_playback_to_animator no longer syncs clip_id (only time, playing, speed, looping)

### Borrowing Issue Resolution
- animation_time_system and skeleton_animation_system: pre-collect resolved clip IDs from ClipSchedule before mutable World borrow
- build_initial_clip_schedule: constructed before entity builder loop to avoid simultaneous mutable/immutable World borrows

---

## Phase B: Multi-Entity Evaluation (2026-01-31)

### Summary
Replaced single-entity `evaluate_animators` with `evaluate_all_animators` that evaluates all animated entities independently. Introduced `AnimationMeta` component to hold per-entity animation type and scale. Removed animation-related fields from global `ModelState` resource. Deleted dead code (`playback_prepare_animations`, `animation_playback_system`).

### New: AnimationMeta Component
- `ecs/component/animation_meta.rs`: holds `animation_type: AnimationType` and `node_animation_scale: f32` per entity
- Registered in World, added `with_animation_meta()` to EntityBuilder
- Attached to each mesh entity in `create_ecs_entities` when animation is present

### New: Single-Mesh Evaluation Functions
- `GraphicsResources::apply_skinning_to_single_mesh()`: applies skinning to one mesh by index
- `GraphicsResources::apply_node_animation_to_single_mesh()`: applies node animation to one mesh by index
- `GraphicsResources::compute_node_global_transforms()` made public for external callers

### evaluate_all_animators
- Replaces `evaluate_animators` (which selected one entity and updated all meshes)
- Iterates all entities with Animator + ClipSchedule + AnimationMeta + MeshRef
- Groups entities by (skeleton_id, clip_id, time, looping) to share pose computation
- Each mesh updated individually via single-mesh functions
- Removed dependency on ModelState and HierarchyState parameters

### ModelState Cleanup
- Removed `animation_type` and `node_animation_scale` fields from ModelState
- ModelState now only holds `has_skinned_meshes` and `model_path`
- `setup_animation_system` no longer sets animation_type/scale on ModelState
- `apply_initial_pose` reads scale from `load_result` directly

### Dead Code Removed
- `playback_prepare_animations` (superseded by evaluate_all_animators)
- `animation_playback_system` in ecs_world_systems.rs (superseded by animation_time_system)
- `find_target_entity` and `resolve_active_clip` private helpers (logic moved into collect_animated_entities)
