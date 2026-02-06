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

## Check Process

1. Read the target files
2. For each file, verify against applicable rules
3. Report violations with file path and line number
4. Suggest corrections

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
