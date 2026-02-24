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

## Testing and Feature Flags

**IMPORTANT**: The `ort` crate (ONNX Runtime) included via the `ml` feature (enabled by default) has CRT initializers
that crash integration test binaries on Windows with `STATUS_ACCESS_VIOLATION`. This only affects integration tests
(`tests/*.rs`), not lib tests (`cargo test --lib`).

**Before running tests**, check `.cargo/config.toml` for test aliases and environment settings (e.g., `ORT_DYLIB_PATH`).

**How to run tests correctly**:

| Command | Description |
|---------|-------------|
| `.\build-with-tests.ps1` | Recommended. Runs lib tests (with ml) and integration tests (without ml) correctly |
| `cargo test --lib` | Lib tests only (144 tests, ml enabled, safe) |
| `cargo test --test ecs_tests --no-default-features` | Integration tests (59 tests, ml disabled, safe) |
| `cargo test --no-default-features` | All tests with ml disabled (reduces functionality but avoids crash) |

**Do NOT run**: `cargo test --test ecs_tests` (without `--no-default-features`) — this will crash.

**If a test crashes with `STATUS_ACCESS_VIOLATION`**: The cause is the `ort` (ONNX Runtime) dependency linked via the
`ml` feature. Add `--no-default-features` to exclude it. See `${IssueHistoryPath}/FbxExportReimportIssues.md`
Issue 4 for details.

## ECS Architecture

**IMPORTANT:** MUST follow architecture rule to add file or code, plan new function.
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

## Robust Coding Guidelines

- **IMPORTANT**: Follow the rules defined in `.claude/rules/coding.md` for all source code.
- These rules are derived from Google C++ Style Guide, C++ Core Guidelines, Unreal Engine Coding Standard, Apple Swift
  API Guidelines, Rust API Guidelines, and Microsoft Rust Guidelines.
- Key principles: make invalid states unrepresentable, validate at boundaries, consistent error handling, RAII, exhaustive
  matching, no boolean parameters, fail fast.

## Path

**IMPORTANT:** All `${...Path}` variables MUST be resolved by reading `.claude/local/paths.md`.
This file contains the absolute paths for this machine. Agents and subagents MUST read it before using any path variable.
Do NOT resolve relative paths manually — always use the absolute paths from `.claude/local/paths.md`.

## Document

**IMPORTANT:** All documents (research, design, issue history, explore history) MUST be saved under
`../SharedData/document/Rust_Rendering/`. Never place documents directly under `../SharedData/document/`.

## Issue History

**IMPORTANT:** If you encounter a complex issue and resolve it, you must document the issue and its solution in detail at
`${IssueHistoryPath}`.

File names must use CamelCase (e.g., ImageLayoutTransition.md).

Each issue must be documented in a separate file, but to avoid huge number of files, try to add issue in a existing file
and recap it.
At the top of each file, include a brief summary of the issue and its resolution to read shortly.

**IMPORTANT:** MUST write in English.

## Last Conversation

- last conversation is saved at .claude/local/last-conversation.md
- **IMPORTANT** MUST read the last conversation file and work continue.

## AnimationTraining
### Repository
repository is separated to ../AnimationModelTraining

### Trained Data

The trained data for the copilot curve is stored in ../SharedData/exports/.

### Interaction Log

-If any issues occur (for example, the training collapses), report them using the Context Memory format so the training
repository can fully understand the situation.
- location at ../SharedData/log/Rendering is rendering side, and log/Training is training side.