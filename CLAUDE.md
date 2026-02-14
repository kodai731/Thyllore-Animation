# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**IMPORTANT**: This file must be written in English for Claude's optimal understanding.
Always respond to the user in user's Claude setting language, but keep this documentation in English.

## Goal

- this project targets Animation + Rendering Engine powered by ECS Architecture.

## Code Format

**IMPORTANT**: Must follow PROJECT_ROOT/rustfmt.toml on coding

## Project Overview

This is a Rust-based Vulkan rendering engine with support for:

- 3D model loading (glTF and FBX formats)
- Skeletal animation, node animation, and morph target animation
- Real-time rendering with Vulkan API
- ImGui integration for debugging UI

## Build and Run Commands

```bash
cargo build                                             # Standard build
cargo test                                              # Run all tests
$env:RUST_LOG="debug"; cargo run --bin rust-rendering   # Run with debug logging
```

**Build with tests (recommended)**:

```powershell
.\build-with-tests.ps1            # Build and run tests, save results to log/log_test.txt
.\build-with-tests.ps1 -Release   # Release build
.\build-with-tests.ps1 -SkipTests # Skip tests
```

## ECS Architecture

** IMPORTANT ** MUST follow architecture rule to add file or code, plan new function.
This project uses an Entity-Component-System (ECS) architecture inspired by [Bevy Engine](https://bevyengine.org/). The
design follows these principles:

### Design Philosophy

1. **Data-Behavior Separation**: Data structures hold only data; all behavior is implemented as system functions
2. **Composition over Inheritance**: Build complex objects by combining simple components
3. **Single Responsibility**: Each component/system has one clear purpose

### Directory Structure

```
src/ecs/
├── component/           # Component definitions (data attached to entities)
├── bundle/              # Common component combinations
├── resource/            # Global dynamic state (changes per frame)
├── systems/             # System functions (behavior/logic)
├── world.rs             # World container for entities and resources
├── query.rs             # Query functions for entity filtering
└── mod.rs
```

### Core Concepts

#### Components

Data-only structs attached to entities. Located in `ecs/component/`.

#### Resources

Global state that changes per frame. Located in `ecs/resource/`. **Only use for dynamic data.**

#### Systems

Pure functions that operate on components and resources. Located in `ecs/systems/`.

```rust
// System function naming convention: <domain>_<action>
pub fn camera_rotate(camera: &mut Camera, delta: Vector2<f32>);
pub fn animation_update(playback: &mut AnimationPlayback, registry: &mut AnimationRegistry, dt: f32);
```

#### Bundles

Predefined component combinations for common entity types. Located in `ecs/bundle/`.

### Query Pattern

Use query functions instead of storing entity IDs:

```rust
pub fn query_grid(world: &World) -> Option<Entity>;
pub fn query_selectable_entities(world: &World) -> Vec<Entity>;
```

### RefCell-Based Interior Mutability

```rust
let camera = app.resource::<Camera>();           // ResRef<Camera> (immutable)
let mut camera = app.resource_mut::<Camera>();   // ResMut<Camera> (mutable)
```

### Adding New Scene Objects

1. Define components in `ecs/component/`
2. Create a bundle in `ecs/bundle/`
3. Add a marker component for queries
4. Implement system functions in `ecs/systems/`
5. Spawn entity with the bundle in initialization

### Reference Projects

- [Bevy Engine](https://github.com/bevyengine/bevy) - Primary reference for ECS patterns
- [Hecs](https://github.com/Ralith/hecs) - Lightweight ECS library
- [Legion](https://github.com/amethyst/legion) - Another Rust ECS implementation

## Single Source of Truth

- **IMPORTANT**: Always follow the Single Source of Truth principle when designing the system.
- For example, skeleton or mesh data in the timeline system, and animation curve data in the UI system, must each have a
  single authoritative source.

## Adding New Models

To load a new model, modify `App::load_model()`:

- For glTF: Update `model_path` variable
- For FBX: Update `model_path_fbx` variable
- Ensure textures are in the same directory or adjust texture paths

## Important Rules

- Always respond in Japanese
- Do NOT perform git write operations (commit, push, etc.)
    - read operation (diff log) is allowed

## Logging

- Use the `log!` macro for logging
- Logs are output to `log/log_N.txt`
- Do NOT output to standard console; use the log files instead

## app/update.rs

- Contains the update processing
- Do NOT write feature-specific processing here
- Instead, write update processing in related files and call them from update

## mod.rs

- **IMPORTANT:** Don't write definition or implementation in mod.rs file
- Only module publish responsibility on mod.rs

## issue history

Issue History Guidelines
**IMPORTANT:** If you encounter an issue and resolve it, you must document the issue and its solution in detail at
.claude/local/IssueHistory/
File names must use CamelCase (e.g., ImageLayoutTransition.md).
Each issue must be documented in a separate file, but to avoid huge number of files, try to add issue in a existing file
and recap it.
At the top of each file, include a brief summary of the issue and its resolution to read shortly.
**IMPORTANT:** You must read all existing issue history files before adding a new one.
**IMPORTANT:** MUST write in English.

## explore history

- explore history and summary reports can be placed at .claude/local/ExploreHistory if necessary.
- MUST write in English.

## Last Conversation

- last conversation is saved at .claude/local/last-conversation.md
- **IMPORTANT** MUST read the last conversation file and work continue.

## AnimationTraining

### Shared Data

The trained data for the copilot curve is stored in ../SharedData/exports/.

### Interaction Log

-If any issues occur (for example, the training collapses), report them using the Context Memory format so the training
repository can fully understand the situation.
- location at ../SharedData/log/Rendering is rendering side, and log/Training is training side.