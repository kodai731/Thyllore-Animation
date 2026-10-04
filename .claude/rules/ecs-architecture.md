---
paths:
  - "src/**"
  - "crates/**"
---

# ECS Architecture

**IMPORTANT:** MUST follow these architecture rules when adding files, code, or planning new features.

This project uses an Entity-Component-System (ECS) architecture. The design follows these principles:

## Design Philosophy

1. **Data-Behavior Separation**: Data structures hold only data; all behavior is implemented as system functions
2. **Composition over Inheritance**: Build complex objects by combining simple components
3. **Single Responsibility**: Each component/system has one clear purpose
4. **Components/Systems Directory Separation**: Within a feature domain, data types and logic are separated
   into `components/` and `systems/` subdirectories (inspired by Unity DOTS and Flecs module patterns)
5. **Small, Atomic Components**: Prefer multiple small components over one large component.
   Components that are always accessed together may share a struct, but data accessed by different systems
   should be in separate components (Flecs design principle: reduces cache misses and unnecessary data loading)
6. **Module Independence**: Feature modules depend on shared component types, not on each other's system
   functions. Inter-module communication happens through components, resources, and events — never through
   direct system-to-system calls across module boundaries (Flecs module design). Shared infrastructure
   (`src/scene/`, `src/hooks/`, `src/app/`, `world.rs`) never names a feature: it consumes registries the
   features subscribe to and reflection the feature's declaration generates (`SceneComponent::TYPE_KEY`,
   `ScalarChannelDomain`, `UiParam`), see `hierarchy.md` "Feature isolation"

## Directory Structure

### Core ECS Layer (`src/ecs/`)

```
src/ecs/
├── component/           # Component definitions (data attached to entities)
├── resource/            # Global dynamic state (changes per frame), one file per resource
├── systems/             # System functions (behavior/logic), one file per domain
│   ├── phases/          # Phase coordinators (execution order); event dispatchers are in phases/event_dispatch/
│   ├── world/           # Engine lifecycle systems (batch run schedule / capture record / report)
│   ├── flame/, water/, wind/  # One directory per effect (spawn, time, preset, object_pick, passes, ...)
│   ├── animation/       # Animation pipeline (collect, evaluate, apply, post_process)
│   ├── curve_copilot/   # ML curve suggestion systems
│   └── world/frame.rs   # run_frame: the phase sequence (FRAME_SCHEDULE)
├── events/              # UiCommand / UiCommandQueue, DialogRequest
├── query/               # Query builder, filters, tuple fetch
├── storage/             # Sparse set component storage
├── registry/            # Component registry (type info)
├── context.rs           # EcsContext: World + assets + per-frame inputs for phases
├── frame_context.rs     # FrameContext: EcsContext + GPU resources (what run_frame takes)
├── effect_context.rs    # EffectContext: what the effect hooks take
├── pass_context.rs      # PassContext: what every RenderPassNode method takes
├── world.rs             # World container for entities and resources
└── mod.rs
```

Component and resource types that an effect exposes as parameters (flame, water, wind, lightning) are declared
once in `crates/thyllore-effect-core` via struct field attributes (`#[derive(thyllore_scene_core::SceneFields)]`, the
proc-macro nested at `crates/thyllore-scene-core/derive/` and re-exported by scene-core, usable by any scene component); `src/ecs/component/`
only wraps them. The effect struct carries `#[scene(key, tag, tags, snapshot, scalars, ui, overwrite, owner?, group?)]`,
a sub-struct carries `#[params(tag, owner?, group?)]`, and a field carries one of `#[persist(owner?, as?, with?,
curve?, debug_range?, renamed_from?, ui(...)?)]`, `#[runtime(ui(...)?)]` or `#[nested]` (`#[nested(runtime)]` for a sub-struct without persisted
fields). The tooltip is the field's `///` doc comment, the label is the title-cased public name, and a `[f32; 3]` with
`ui(...)` is a `Color` unless `kind = Absorption` / `Offset` says otherwise; no generated JSON is checked in. The scene form is nested by struct; the public parameter
name is the underscore-joined path (`noise.amplitude` → `noise_amplitude`) and is the one string used by
`ScalarChannel.cli_name`, the batch CLI, MCP and the Blender property identifier. `#[scene(key = ...)]` makes the
component a `thyllore_scene_core::SceneComponent` (type key + persisted field list), which is all the scene format
needs: it never names the effect (see `hierarchy.md`, "Feature isolation").

### Domain ECS Modules

Feature-specific domains that have their own data/logic separation follow the `components/` and `systems/`
pattern within their module directory. The editable animation domain lives in the pure crate
`crates/thyllore-anim-core` (re-exported as `src/animation.rs`); it has no ECS dependency.

```
crates/thyllore-anim-core/src/editable/
├── components/          # Data types (structs, enums, type aliases)
│   ├── keyframe.rs      # EditableKeyframe, BezierHandle, TangentType, etc.
│   ├── curve.rs         # PropertyCurve, PropertyType (data + accessors only)
│   ├── track.rs         # BoneTrack (data + accessors only)
│   ├── clip.rs          # EditableAnimationClip (data + accessors only)
│   ├── clip_file.rs     # Clip file record
│   ├── blend.rs         # BlendMode, EaseType
│   ├── clip_instance.rs # ClipInstance
│   ├── clip_group.rs    # ClipGroup, ClipGroupId
│   ├── source_clip.rs   # SourceClip
│   ├── keyframe_copy.rs # Clipboard record
│   ├── snap_settings.rs # Snap settings
│   ├── mirror.rs        # MirrorAxis, MirrorMapping (data types only)
│   └── mod.rs
│
├── systems/             # Logic and operations (pure functions)
│   ├── curve_ops.rs     # curve_sample, curve_add_keyframe, curve_sort_keyframes
│   ├── clip_ops.rs      # clip-level edits
│   ├── tangent.rs       # sample_bezier, apply_auto_tangent, apply_tangent_by_type
│   ├── clip_convert.rs  # from_animation_clip, to_animation_clip
│   ├── bake.rs          # bake constraints / spring bones into keyframes
│   ├── mirror.rs        # build_mirror_mapping, mirror_keyframes
│   ├── snap.rs          # snap_time, compute_snap_threshold_time
│   ├── manager.rs       # EditableClipManager (I/O and lifecycle management)
│   └── mod.rs
│
└── mod.rs
```

The same split is used by `crates/thyllore-effect-core` (`flame/`, `water/`, `wind/`: `effect/` data,
`analytic/` pure math, `gpu/` UBO structs, `presets.rs`, `settings.rs`).

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

A resource that belongs in the scene file declares its persisted fields with `declare_scene_format!`
(runtime-only fields stay out of the table) and registers with `scene_resource!(Type)` in its own file;
`src/scene/` never mirrors it (see `hierarchy.md`, "Feature isolation").

Resources correspond to **singleton components** in other ECS frameworks:
- Flecs: singletons (component added to its own entity)
- Unity DOTS: singleton components (`GetSingleton<T>()`)
- EnTT: context variables (`registry.ctx()`)

### Systems

Pure functions that operate on components and resources. Located in `ecs/systems/`.

```rust
pub fn camera_rotate(camera: &mut Camera, delta: Vector2<f32>);
pub fn animation_update(playback: &mut AnimationPlayback, registry: &mut AnimationRegistry, dt: f32);
```

### Spawning

There is no `bundle/` directory. Entities are spawned by a `spawn_*` system in the owning domain
(`src/ecs/systems/flame/spawn.rs`, `water/spawn.rs`, `mesh_systems.rs`, ...) which inserts the component set.

## Phase Pipeline

`FRAME_SCHEDULE` (`src/ecs/systems/world/frame.rs`) is the one list of phases. `App::drive_frame`
(`src/app/frame.rs`, called from `src/platform/events/frame.rs` once the imgui frame is built) runs it:
EventDispatch first, then `begin_frame`, the update phases through `run_frame()`, the render, and Last
once the image is presented. The slots are:

```
EventDispatch → run_event_dispatch_phase()   # DispatchPrepHooks (worker polls), UI commands → World; dialog requests; apply commands
First         → run_first_phase()            # FrameClock.frame += 1, batch schedule
Input         → run_input_phase()            # Input handling, gizmo interaction
Transform     → run_transform_phase_ecs()    # Camera, light gizmo, billboard (entity transforms: #195)
Timeline      → run_timeline_phase()         # Timeline / clip schedule advance
Animation     → run_animation_phase_ecs()    # Animation evaluation, blending, transform propagation
              → run_animation_phase_gpu()    # Skinning / vertex upload
OnionSkin     → run_onion_skin_phase()       # Ghost frame generation
RenderPrep    → run_transform_phase_gpu()    # Light gizmo vertex upload (#194)
              → run_render_prep_phase()      # Uniforms, gizmo meshes, frame prep hooks, TLAS refresh
                (App::render: record, submit, present)
Last          → run_last_phase()             # Requested BatchCapture readbacks + scheduled screenshot
```

`run_frame()` runs `update_phases()`, the slots between EventDispatch and Last; `App::drive_frame`
owns the two ends because they need `App` (file dialogs and queued commands before, the presented image
after).

### Phase Design Principles (from Flecs, Unity DOTS, Bevy)

- **Phases are sequential**: Each phase completes before the next begins
- **Systems within a phase may have internal ordering**: Expressed through call sequence in the phase
  coordinator function
- **Animation completes before Transform propagation**: Standard in all major engines
  (Bevy: `.before(TransformSystems::Propagate)`, Unity DOTS: animation in SimulationSystemGroup
  before TransformSystemGroup)
- **UI events are applied before the update, outputs to the platform after it**: the UI is immediate
  mode, so the events it recorded are dispatched into `World` at the start of the frame and the update
  sees them the same frame (Bevy: input and `bevy_egui` input in `PreUpdate`, Unreal: the message pump
  routes Slate input before `UWorld::Tick`). The file dialogs the engine asks of the platform are
  `DialogRequest`s that `src/platform/events/file_dialog.rs` drains right after the dispatch; what `App` must do is pushed to a command queue and applied by `App` before
  `begin_frame`; readbacks of the finished image are Last (Bevy `PostUpdate` egui output, Unreal
  `ProcessLocalPlayerSlateOperations` after the world tick)
- **Commands are queued by stage, not returned**: a dispatcher or system pushes to the `CommandQueue<C>`
  resource of its command type (`src/ecs/resource/app_command.rs`) and never returns commands to its
  caller. One queue exists per stage whose order is required, and `App` drains them in that order
  (`src/app/command.rs::apply_queued_commands`): `EntityRemovalQueue` (ids of the scene before any load)
  → `SceneLoadQueue` (model loads, spawns) → `AssetEditQueue` (edits of the loaded model's assets) →
  `OutputQueue` (screenshots, dumps, saves, exports, which only read). Inside a queue the order is
  unspecified; a command that needs a new ordering guarantee gets a new queue, not a position in a list

### Adding New Phases

Add a new variant to `FramePhase`, place it in `FRAME_SCHEDULE`, and create its coordinator in
`phases/<name>_phase.rs`.

## Query Pattern

Use query functions instead of storing entity IDs:

```rust
pub fn query_grid(world: &World) -> Option<Entity>;
pub fn query_selectable_entities(world: &World) -> Vec<Entity>;
```

For frequently called queries, consider caching results in a resource to avoid repeated iteration
(analogous to Flecs cached queries vs uncached queries).

## RefCell-Based Interior Mutability

```rust
let camera = app.resource::<Camera>();           // ResRef<Camera> (immutable)
let mut camera = app.resource_mut::<Camera>();   // ResMut<Camera> (mutable)
```

## Adding New Scene Objects

1. Define components in `ecs/component/` (effect parameters: declare them in `thyllore-effect-core`)
2. Add a marker component for queries
3. Implement system functions in `ecs/systems/<domain>/`
4. Add a `spawn_*` system (a thin wrapper over `hooks::scene::spawn_scene_owner`) and call it from the
   event dispatcher or initialization; runtime-only companions (baked data, accumulators) are inserted by
   the domain's per-frame system when missing, so a loaded entity and a spawned one converge
5. Persist it: give the effect parameter component `#[scene(key = ...)]` (resource: `declare_scene_format!`) and write
   `scene_owner!(C { icon, placement, prepare_loaded? })` in its `ecs/component/` file; provenance
   components (applied preset / style) implement `SceneComponent` and write `scene_attachment!(P)`.
   Registration happens at link time; neither `src/scene/` nor `subscription.rs` is edited. For an
   animatable scalar, add `curve` to the field's `#[persist(...)]` (effect-core); nothing in `src/`
   changes and no number is assigned (see `ui.md`).

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

### Dependency Direction

Following Flecs module design, dependencies flow in one direction:

```
Platform Layer (src/platform/, src/app/)
    │  reads resources, sends events, calls phase entry points
    ▼
ECS Systems Layer (src/ecs/systems/)
    │  calls pure domain functions, operates on components/resources
    ▼
Domain Layer (crates/thyllore-anim-core, thyllore-effect-core, thyllore-model-core, ...)
    │  pure data types and pure functions, no World dependency
    ▼
Core Types (src/ecs/component/, src/ecs/resource/)
    data definitions only
```

**Allowed in platform layer** (`src/platform/`):
- Reading resources for UI display (immutable access)
- Sending UI commands with `World::send_command` and dialog requests with `send_dialog_request`
- Calling a single ECS dispatch entry point (e.g., `run_event_dispatch_phase`)
- Platform-specific I/O (file dialogs, window management, imgui orchestration)
- Converting file dialog results into commands and pushing them to their `CommandQueue` (applied by `App`,
  never by the platform layer)

**NOT allowed in platform layer**:
- Directly calling multiple ECS system functions to process events
- Match-dispatching UI command variants to mutate `World`/`AssetStorage` (that is the command's `apply`)
- Implementing event handler logic inline
- Using `resource_mut` or `get_component_mut` for business logic mutations

### Domain Layer Independence

Domain crates (`crates/thyllore-anim-core`, `thyllore-effect-core`, ...) must be **pure** — no dependency on
`World`, `Entity`, `AssetStorage`, or `GraphicsResources`. This enables:
- Unit testing without ECS infrastructure
- Reuse across different ECS contexts
- Clear separation between "what the data is" and "how the engine uses it"

## Event System

UI input follows a **record-then-apply** pattern (similar to Unity DOTS EntityCommandBuffer and Flecs
deferred events):

1. **Platform layer** sends commands with `World::send_command(command)` (immutable World access only)
2. **Event dispatch phase** drains the one `UiCommandQueue` and calls `apply` on each command, in send order
3. **Structural changes** (entity creation/destruction) happen only during that apply

A UI command is a per-feature enum that implements `UiCommand` (`src/ecs/events/ui_command.rs`:
`apply(self: Box<Self>, world, assets, graphics)`), declared in the file that applies it
(`src/ecs/systems/phases/event_dispatch/<feature>.rs`; a feature behind a cargo feature keeps the whole file under
`#![cfg(...)]`, see `event_dispatch/ml/`). Adding a UI interaction touches that file (variant + match arm) and the
sending window, nothing else: the phase never names a feature, there is no registration and no stage. Two commands
that must apply in order within one frame are sent in that order (or merged into one command); the queue is FIFO.
File-dialog requests are not commands: the window sends `DialogRequest` (`src/ecs/events/dialog_request.rs`,
`send_dialog_request`) and `src/platform/events/file_dialog.rs` drains them after the phase.

Editor windows are the other half: each file in `src/platform/ui/` registers its window with `ui_window!`
(`src/hooks/ui_window.rs`), reads `LayoutSnapshot` / `ViewportInput` and its own state resource from `World`, and
sends commands. When to use a hook and when a command is decided in `hierarchy.md`, "Hook or command?".

## Bones are NOT Entities

- Bones are data within `Skeleton.bones: Vec<Bone>`
- Bones identified by `BoneId` (u32 index), not `Entity`
- Constraints reference bones via `BoneId`

This is a deliberate design choice: skeleton data is a small, structured hierarchy that benefits from
contiguous memory (Vec) rather than ECS entity overhead. Flecs also recognizes this pattern —
small structural hierarchies can use optimized storage rather than full entity relationships.

## Reference Projects

### Tier 1: Industry Standard (Production-Proven)

- **[Unity DOTS (Entities)](https://docs.unity3d.com/Packages/com.unity.entities@1.0/manual/)**
  - Archetype ECS with chunk-based memory layout
  - 3 root SystemGroups: Initialization → Simulation → Presentation
  - `UpdateBefore`/`UpdateAfter` attributes for system ordering within groups
  - EntityCommandBuffer for deferred structural changes (record → playback)
  - Singleton components for global state (equivalent to this project's Resources)
  - Baker/Authoring pattern: separates editor data from runtime ECS data
  - **Primary reference for**: phase ordering, deferred action pattern, directory layout

- **[Unreal Mass Entity](https://dev.epicgames.com/documentation/en-us/unreal-engine/mass-entity-in-unreal-engine)**
  - Plugin-per-feature (MassMovement, MassRepresentation)
  - Fragments (Components) and Processors (Systems) co-located per plugin
  - **Primary reference for**: feature-per-plugin organization

### Tier 2: High-Quality ECS Libraries

- **[Flecs](https://github.com/SanderMertens/flecs)** (C/C++, ~7k stars)
  - Archetype ECS with 8 built-in pipeline phases
  - `DependsOn` relationships for topological sort of phase ordering
  - Module design: components are shared types; systems depend only on components, not other systems
  - Small atomic components recommended (Position + Rotation + Scale, not Transform)
  - Relationship system (ChildOf, IsA) with cleanup traits
  - Observer system with lifecycle events (OnAdd, OnRemove, OnSet)
  - [Design with Flecs](https://www.flecs.dev/flecs/md_docs_2DesignWithFlecs.html) — essential reading
  - **Primary reference for**: module boundary design, phase pipeline, component granularity

- **[EnTT](https://github.com/skypjack/entt)** (C++, ~10k stars)
  - Sparse set ECS (vs archetype): O(1) component add/remove, fast single-component queries
  - Registry as coordinator (not monolithic container)
  - View (flexible, no ownership) vs Group (high-performance, owns component pools)
  - Signal system: on_construct, on_update, on_destroy per component type
  - Context variables for singleton/resource data
  - No prescribed project structure — library, not framework
  - **Primary reference for**: sparse set trade-offs, signal/event patterns, minimal API design

- **[Bevy Engine](https://github.com/bevyengine/bevy)** (Rust, ~44k stars)
  - Archetype ECS with Schedule + SystemSet for execution ordering
  - Feature-per-crate, data and systems co-located within each crate
  - Animation pipeline: 6 chained systems in PostUpdate, before TransformSystems::Propagate
  - Plugin architecture for modular feature registration
  - **Primary reference for**: Rust ECS patterns, animation pipeline phases, system ordering

### Tier 3: Specialized References

- **[Fyrox](https://github.com/FyroxEngine/Fyrox)** (Rust)
  - Scene graph + Generational Arena (Pool), not pure ECS
  - Animation Blending State Machine (ABSM): layers + states + transitions + blend nodes
  - Each layer has its own state machine; all layers blend to final pose
  - Plugin system with hot-reload support
  - **Primary reference for**: animation blending architecture (ABSM, layers)

- **[Hecs](https://github.com/Ralith/hecs)** (Rust, ~1.2k stars)
  - Minimal archetype ECS. Clean, small codebase
  - **Primary reference for**: understanding core ECS internals

### Architecture Comparison

| Feature | This Project | Flecs | Unity DOTS | EnTT | Bevy |
|---------|-------------|-------|------------|------|------|
| Storage | Sparse Set (`src/ecs/storage/`) | Archetype | Archetype (Chunk) | Sparse Set | Archetype |
| Data/Logic Split | Separate dirs | Separate modules | Flat per feature | Free-form | Mixed per crate |
| Organization Unit | Directory | Module | Directory | Free-form | Crate |
| Phase System | Coordinator fns | DependsOn pipeline | SystemGroup hierarchy | User-defined | Schedule + SystemSet |
| Global State | Resource | Singleton | Singleton Component | Context Variable | Resource |
| Events | `UiCommandQueue` (FIFO of `Box<dyn UiCommand>`) | Observer + emit | ECB + SystemGroup | Signal (sigh/sink) | Event\<T\> + EventReader |
| Deferred Changes | `CommandQueue<C>` per stage | Sync point flush | EntityCommandBuffer | - | Commands |

### Key Patterns Adopted from Each

| Pattern | Source | Implementation in This Project |
|---------|--------|-------------------------------|
| components/ + systems/ directory split | Flecs, Unity DOTS | `thyllore-anim-core/src/editable/components/` + `systems/` |
| Phase pipeline with explicit ordering | Flecs, Unity DOTS, Bevy | `phases/` directory with coordinator functions |
| Resources for global state | All (unanimous) | `ecs/resource/` |
| Record-then-dispatch events | Unity DOTS (ECB), Flecs (deferred) | `UiCommandQueue` → `event_dispatch_phase` |
| Module depends on types only, not logic | Flecs | Platform layer reads resources, sends events only |
| Pure domain layer (no World dependency) | Bevy (per-crate), Flecs (module independence) | `thyllore-anim-core` / `thyllore-effect-core` have no ECS dependency |
| Contiguous memory for bulk data | Flecs, Bevy, Unreal | `Vec<Bone>`, `Vec<Keyframe>` for animation data |
| Animation before Transform propagation | Bevy, Unity DOTS | `run_animation_phase` before `run_transform_phase` |
