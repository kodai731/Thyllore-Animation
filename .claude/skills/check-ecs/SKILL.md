---
name: check-ecs
description: Check if code follows ECS architecture rules defined in CLAUDE.md. Use when writing or reviewing systems, components, resources, or any ECS-related code.
user-invocable: true
allowed-tools: Read, Grep, Glob
---

# ECS Architecture Compliance Check

Check the specified files or recent changes against project ECS rules.

this skill is called as
```
/check-ecs
```

Or with file name as arguments
```
/check-ecs file_name
```

MUST use ecs-architecture-checker agent to explore.

## Target

If `$ARGUMENTS` is provided, check those files. Otherwise, check recently modified `.rs` files.

## ECS Rules to Verify

### 1. Component Rules (`ecs/component/`)

- [ ] Components are **data-only structs** — no impl blocks with logic
- [ ] No `fn` methods that perform computation or mutation
- [ ] Derive macros only (Debug, Clone, Default, etc.)
- [ ] Located in `src/ecs/component/`

**Violation example:**
```rust
// BAD: Component with logic
impl MyComponent {
    pub fn calculate_something(&self) -> f32 { ... }
}
```

### 2. System Rules (`ecs/systems/`)

- [ ] All behavior implemented as **free functions**
- [ ] Function naming: `<domain>_<action>` (e.g., `camera_rotate`, `animation_update`)
- [ ] Takes World/Components/Resources as parameters
- [ ] Located in `src/ecs/systems/`

**Violation example:**
```rust
// BAD: Logic in component file
// src/ecs/component/camera.rs
impl Camera {
    pub fn rotate(&mut self, delta: Vector2) { ... }
}

// GOOD: Logic in system file
// src/ecs/systems/camera_systems.rs
pub fn camera_rotate(camera: &mut Camera, delta: Vector2) { ... }
```

### 3. Resource Rules (`ecs/resource/`)

- [ ] Resources are **global dynamic state** that changes per frame
- [ ] Static configuration should NOT be a resource
- [ ] Located in `src/ecs/resource/`

### 4. mod.rs Rules

- [ ] `mod.rs` files contain **ONLY module declarations and re-exports**
- [ ] No struct/enum definitions in mod.rs
- [ ] No function implementations in mod.rs

**Violation example:**
```rust
// BAD: Definition in mod.rs
// src/ecs/component/mod.rs
pub struct MyComponent { ... }

// GOOD: Only re-exports
// src/ecs/component/mod.rs
mod my_component;
pub use my_component::MyComponent;
```

### 5. Bones are NOT Entities

- [ ] Bones are data within `Skeleton.bones: Vec<Bone>`
- [ ] Bones identified by `BoneId` (u32 index), not `Entity`
- [ ] Constraints reference bones via `BoneId`

### 6. EntityBuilder Pattern

- [ ] Use existing `EntityBuilder` for entity construction
- [ ] No manual component insertion sequences
- [ ] Located in `src/ecs/world.rs`

### 7. Layer Boundary Rules

ECS business logic MUST live inside `src/ecs/systems/`. Code outside `src/ecs/`
(especially `src/platform/`, `src/app/`) must NOT contain ECS domain logic.

**Allowed in platform layer** (`src/platform/`):
- Reading resources for UI display (`resource::<T>()`, `get_resource::<T>()`)
- Sending events to `UIEventQueue` (`ui_events.send(...)`)
- Calling a single ECS dispatch entry point (e.g., `run_event_dispatch_phase(...)`)
- Platform-specific I/O (file dialogs via `rfd`, window management, imgui frame orchestration)
- Handling `DeferredAction` that requires `App`/`GUIData` (model loading, screenshots)

**NOT allowed in platform layer**:
- Directly calling multiple ECS system functions to process events
- Match-dispatching `UIEvent` variants to mutate `World`/`AssetStorage`
- Implementing event handler logic inline (e.g., `process_*_events_inline`)
- Accessing `resource_mut::<T>()` or `get_component_mut::<T>()` for business logic

**Detection patterns** (search in files outside `src/ecs/`):
```rust
// Suspicious: multiple resource_mut calls with business logic
world.resource_mut::<ClipLibrary>();
world.resource_mut::<TimelineState>();
world.get_component_mut::<ClipSchedule>(entity);

// Suspicious: match on UIEvent with mutation logic
match event {
    UIEvent::TimelinePlay => { /* logic here */ }
    UIEvent::Undo => { /* logic here */ }
}
```

**Violation example:**
```rust
// BAD: ECS logic in platform layer
// src/platform/events.rs
fn process_timeline_events(events: &[UIEvent], app: &mut App) {
    let mut timeline = app.data.ecs_world.resource_mut::<TimelineState>();
    let mut clips = app.data.ecs_world.resource_mut::<ClipLibrary>();
    timeline_process_events(events, &mut timeline, &mut clips);
}

// GOOD: Platform calls single ECS entry point
// src/platform/events.rs
let (platform_events, deferred) = run_event_dispatch_phase(
    &mut app.data.ecs_world,
    &mut app.data.ecs_assets,
    model_bounds,
);

// GOOD: ECS logic in ECS layer
// src/ecs/systems/phases/event_dispatch_phase.rs
fn dispatch_timeline_events(events: &[UIEvent], world: &mut World) { ... }
```

**Scope**: This rule applies to ALL `.rs` files outside `src/ecs/`, not just
recently modified ones. When running `/check-ecs` without arguments, also scan
`src/platform/` and `src/app/` for layer violations.

## Check Process

1. Read the target files
2. For each file, verify against applicable rules
3. **If no arguments given**: also scan `src/platform/*.rs` and `src/app/*.rs`
   for Rule 7 (layer boundary) violations
4. Report violations with file path and line number
5. Suggest corrections

## Output Format

```
## ECS Check Results

### ✅ Compliant
- file.rs: Components are data-only

### ❌ Violations
- src/ecs/component/foo.rs:45 - Component has logic method `calculate()`
  Suggestion: Move to `src/ecs/systems/foo_systems.rs`

- src/ecs/component/mod.rs:12 - Struct definition in mod.rs
  Suggestion: Move to separate file and re-export
```
