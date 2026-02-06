# Context Memory

## Project / Topic
- Rust Vulkan rendering engine, ECS architecture, branch feature/rig-system
- Phase 8: Bake-to-Keyframes Export implementation

## Goals
- Evaluate all frames of constrained animation, record TRS per bone as keyframes
- Generate constraint-free EditableAnimationClip and register in ClipLibrary

## Key Decisions
- Created constraint_bake_systems.rs with 3 public functions: constraint_bake_evaluate, constraint_bake_rest_pose, constraint_bake_register
- UIEvent::ConstraintBakeToKeyframes triggers bake via process_constraint_bake_events_inline in events.rs
- ConstraintEditorState holds bake_fps (default 30.0), exposed in constraint_inspector UI
- Fallback to rest pose bake when no clip is selected

## Constraints & Rules
- All 8 files modified/created per plan; build succeeds; 67 unit tests pass
- ECS architecture checker confirmed full compliance of new code

## Open Questions
- Runtime verification pending: load model, add constraint, press Bake, confirm baked clip appears in ClipBrowser
