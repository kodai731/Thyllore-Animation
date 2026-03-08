# ECS Architecture

**IMPORTANT:** MUST follow these architecture rules when adding files, code, or planning new features.

This project uses an Entity-Component-System (ECS) architecture. The design follows these principles:

## Design Philosophy

1. **Data-Behavior Separation**: Data structures hold only data; all behavior is implemented as system functions
2. **Composition over Inheritance**: Build complex objects by combining simple components
3. **Single Responsibility**: Each component/system has one clear purpose
4. **Components/Systems Directory Separation**: Within a feature domain, data types and logic are separated
   into `components/` and `systems/` subdirectories (inspired by Unity DOTS and Flecs module patterns)

## Directory Structure

### Core ECS Layer (`src/ecs/`)

```
src/ecs/
├── component/           # Component definitions (data attached to entities)
├── bundle/              # Common component combinations
├── resource/            # Global dynamic state (changes per frame)
├── systems/             # System functions (behavior/logic)
├── events/              # Event definitions
├── world.rs             # World container for entities and resources
├── query.rs             # Query functions for entity filtering
└── mod.rs
```

### Domain ECS Modules

Feature-specific domains that have their own data/logic separation follow the `components/` and `systems/`
pattern within their module directory.

```
src/animation/editable/
├── components/          # Data types (structs, enums, type aliases)
│   ├── keyframe.rs      # EditableKeyframe, BezierHandle, TangentType, etc.
│   ├── curve.rs         # PropertyCurve, PropertyType (data + accessors only)
│   ├── track.rs         # BoneTrack (data + accessors only)
│   ├── clip.rs          # EditableAnimationClip (data + accessors only)
│   ├── blend.rs         # BlendMode, EaseType
│   ├── clip_instance.rs # ClipInstance
│   ├── clip_group.rs    # ClipGroup, ClipGroupId
│   ├── source_clip.rs   # SourceClip
│   ├── mirror.rs        # MirrorAxis, MirrorMapping (data types only)
│   └── mod.rs
│
├── systems/             # Logic and operations (pure functions)
│   ├── curve_ops.rs     # curve_sample, curve_add_keyframe, curve_sort_keyframes
│   ├── tangent.rs       # sample_bezier, apply_auto_tangent, apply_tangent_by_type
│   ├── clip_convert.rs  # from_animation_clip, to_animation_clip
│   ├── mirror.rs        # build_mirror_mapping, mirror_keyframes
│   ├── snap.rs          # snap_time, compute_snap_threshold_time
│   ├── manager.rs       # EditableClipManager (I/O and lifecycle management)
│   └── mod.rs
│
└── mod.rs
```

### What Goes in `components/`

- Struct and enum definitions (data types)
- `new()`, `Default`, `Clone`, `Serialize`/`Deserialize` derives
- Simple accessors: `get_*()`, `set_*()`, field access helpers
- Computed properties that read but do not mutate (e.g., `effective_duration()`, `is_empty()`)
- Collection helpers on owned data (e.g., `contains_instance()`, `add_instance()` for Vec operations)

### What Goes in `systems/`

- Pure functions that compute, transform, or sample data (e.g., `curve_sample`, `sample_bezier`)
- Functions that mutate state across multiple data structures (e.g., `recalculate_tangent_at`)
- Format conversion functions (e.g., `from_animation_clip`, `to_animation_clip`)
- I/O operations (e.g., `save_to_file`, `load_from_file`)
- Functions that coordinate operations across multiple components

## Core Concepts

### Components

Data-only structs attached to entities. Located in `ecs/component/`.

### Resources

Global state that changes per frame. Located in `ecs/resource/`. **Only use for dynamic data.**

### Systems

Pure functions that operate on components and resources. Located in `ecs/systems/`.

```rust
// System function naming convention: <domain>_<action>
pub fn camera_rotate(camera: &mut Camera, delta: Vector2<f32>);
pub fn animation_update(playback: &mut AnimationPlayback, registry: &mut AnimationRegistry, dt: f32);
```

### Bundles

Predefined component combinations for common entity types. Located in `ecs/bundle/`.

## Query Pattern

Use query functions instead of storing entity IDs:

```rust
pub fn query_grid(world: &World) -> Option<Entity>;
pub fn query_selectable_entities(world: &World) -> Vec<Entity>;
```

## RefCell-Based Interior Mutability

```rust
let camera = app.resource::<Camera>();           // ResRef<Camera> (immutable)
let mut camera = app.resource_mut::<Camera>();   // ResMut<Camera> (mutable)
```

## Adding New Scene Objects

1. Define components in `ecs/component/`
2. Create a bundle in `ecs/bundle/`
3. Add a marker component for queries
4. Implement system functions in `ecs/systems/`
5. Spawn entity with the bundle in initialization

## Adding New Domain Features

When a feature domain has enough data types and logic to warrant separation:

1. Create `<domain>/components/` for data types
2. Create `<domain>/systems/` for logic functions
3. Keep `<domain>/mod.rs` for re-exports only
4. Data types must not depend on system functions
5. System functions import and operate on data types

## Layer Boundary Rules

**IMPORTANT:** ECS business logic MUST live inside `src/ecs/systems/`. Code outside `src/ecs/` (especially
`src/platform/`, `src/app/`) must NOT contain ECS domain logic.

**Allowed in platform layer** (`src/platform/`):
- Reading resources for UI display (immutable access)
- Sending events to `UIEventQueue`
- Calling a single ECS dispatch entry point (e.g., `run_event_dispatch_phase`)
- Platform-specific I/O (file dialogs, window management, imgui orchestration)
- Handling `DeferredAction` that requires `App`/`GUIData`

**NOT allowed in platform layer**:
- Directly calling multiple ECS system functions to process events
- Match-dispatching `UIEvent` variants to mutate `World`/`AssetStorage`
- Implementing event handler logic inline
- Using `resource_mut` or `get_component_mut` for business logic mutations

## Bones are NOT Entities

- Bones are data within `Skeleton.bones: Vec<Bone>`
- Bones identified by `BoneId` (u32 index), not `Entity`
- Constraints reference bones via `BoneId`

## Reference Projects

### Rust ECS

- [Bevy Engine](https://github.com/bevyengine/bevy) - Feature-per-crate, data and systems co-located
  within each crate. Primary reference for this project's core ECS patterns.
- [Hecs](https://github.com/Ralith/hecs) - Lightweight ECS library
- [Legion](https://github.com/amethyst/legion) - Another Rust ECS implementation

### C/C++ ECS

- [Flecs](https://github.com/SanderMertens/flecs) - Recommends `components.*` and `systems.*` module
  separation per feature domain. Primary reference for this project's domain module organization.
- [EnTT](https://github.com/skypjack/entt) - Header-only C++ ECS. No prescribed structure; users organize
  freely. Systems are plain functions/lambdas.

### C# / Engine ECS

- [Unity DOTS (Entities)](https://github.com/Unity-Technologies/EntityComponentSystemSamples) - Uses
  `Components/`, `Systems/`, `Authoring/` directory separation. Primary reference for directory layout.
- [Unreal Mass Entity](https://dev.epicgames.com/documentation/en-us/unreal-engine/mass-entity-in-unreal-engine) -
  Plugin-per-feature (MassMovement, MassRepresentation), Fragments and Processors co-located per plugin.

### Pattern Summary

| Project        | Data/Logic Separation     | Organization Unit     |
|----------------|---------------------------|-----------------------|
| Bevy           | Mixed per file            | Crate (feature)       |
| Flecs          | Separate modules          | Module (feature)      |
| Unity DOTS     | Separate directories      | Directory (feature)   |
| Unreal Mass    | Mixed per plugin          | Plugin (feature)      |
| EnTT           | User-defined              | Free-form             |
| **This project** | **Separate directories** | **Directory (feature)** |
