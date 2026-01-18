# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**IMPORTANT**: This file must be written in English for Claude's optimal understanding. Always respond to the user in Japanese, but keep this documentation in English.

## Project Overview

This is a Rust-based Vulkan rendering engine with support for:
- 3D model loading (glTF and FBX formats)
- Skeletal animation, node animation, and morph target animation
- Real-time rendering with Vulkan API
- ImGui integration for debugging UI

## Build and Run Commands

### Building the Project

**Standard build**:
```bash
cargo build
```

**Build with tests (recommended)**:
```powershell
# Build and run tests sequentially, save results to log/log_test.txt
.\build-with-tests.ps1

# Release build
.\build-with-tests.ps1 -Release

# Skip tests
.\build-with-tests.ps1 -SkipTests
```

This script:
1. Builds the project
2. Runs all tests after successful build
3. Saves test results to `log/log_test.txt`

### Running the Application
```bash
# With debug logging (recommended for development)
$env:RUST_LOG="debug"; cargo run --bin rust-rendering

# Without logging
cargo run --bin rust-rendering
```

**Note**: When running `cargo run`, automatic tests are skipped and the application starts normally.

### Compiling Shaders
Shaders are automatically compiled during `cargo build`. The build system compiles all shader files from `shaders/` directory to `assets/shaders/` using glslc from VulkanSDK.

Shader source files are located in `shaders/`:
- `vertex.vert` → `assets/shaders/vert.spv`
- `fragment.frag` → `assets/shaders/frag.spv`
- `gbufferVertex.vert` → `assets/shaders/gbufferVert.spv`
- `gbufferFragment.frag` → `assets/shaders/gbufferFrag.spv`
- `rayQueryShadow.comp` → `assets/shaders/rayQueryShadow.spv`
- etc.

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test integration_tests
cargo test --test model_loading_tests
cargo test --test shader_tests

# Run tests with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

## ECS Architecture

This project uses an Entity-Component-System (ECS) architecture inspired by [Bevy Engine](https://bevyengine.org/). The design follows these principles:

### Design Philosophy

1. **Data-Behavior Separation**: Data structures hold only data; all behavior is implemented as system functions
2. **Composition over Inheritance**: Build complex objects by combining simple components
3. **Single Responsibility**: Each component/system has one clear purpose

### Directory Structure

```
src/ecs/
├── component/           # Component definitions (data attached to entities)
│   ├── mod.rs
│   ├── core.rs          # Name, Transform, Visible
│   ├── mesh.rs          # GpuMesh, GizmoVertices, MeshRef
│   ├── render.rs        # PipelineRef, ObjectIndex
│   ├── interaction.rs   # Selectable, Draggable
│   └── marker.rs        # Tag components: Grid, LightGizmo, LightBillboard
├── bundle/              # Common component combinations
│   ├── mod.rs
│   ├── grid.rs          # GridBundle
│   ├── gizmo.rs         # LightGizmoBundle
│   └── billboard.rs     # BillboardBundle
├── resource/            # Global dynamic state (changes per frame)
│   ├── mod.rs
│   ├── animation_playback.rs
│   ├── graphics.rs
│   ├── model_info.rs
│   └── pipeline_manager.rs
├── systems/             # System functions (behavior/logic)
│   ├── mod.rs
│   ├── camera_systems.rs
│   ├── animation_systems.rs
│   ├── light_gizmo_systems.rs
│   ├── model_systems.rs
│   └── render_data_systems.rs
├── world.rs             # World container for entities and resources
├── query.rs             # Query functions for entity filtering
└── mod.rs
```

### Core Concepts

#### Components

Data-only structs attached to entities. Located in `ecs/component/`.

```rust
// Core components
pub struct Name(pub String);
pub struct Transform { pub position: Vector3<f32>, pub rotation: Quaternion<f32>, pub scale: Vector3<f32> }
pub struct Visible(pub bool);

// Marker components (tag only, no data)
pub struct Grid;           // Identifies grid entity
pub struct LightGizmo;     // Identifies light gizmo entity
pub struct LightBillboard; // Identifies billboard entity
```

#### Resources

Global state that changes per frame. Located in `ecs/resource/`. **Only use for dynamic data.**

```rust
// Good: Dynamic state that changes during runtime
pub struct Camera { pub position: Vector3<f32>, pub direction: Vector3<f32>, ... }
pub struct AnimationPlayback { pub current_time: f32, pub is_playing: bool, ... }
pub struct Time { pub delta: f32, pub elapsed: f32 }

// Bad: Static configuration (should be component or const)
// pub struct WindowSize { width: u32, height: u32 }  // <- Don't do this
```

#### Systems

Pure functions that operate on components and resources. Located in `ecs/systems/`.

```rust
// System function naming convention: <domain>_<action>
pub fn camera_rotate(camera: &mut Camera, delta: Vector2<f32>);
pub fn camera_zoom(camera: &mut Camera, amount: f32);
pub fn light_gizmo_try_select(world: &World, ray: Ray) -> Option<Entity>;
pub fn animation_update(playback: &mut AnimationPlayback, registry: &mut AnimationRegistry, dt: f32);
```

#### Bundles

Predefined component combinations for common entity types. Located in `ecs/bundle/`.

```rust
pub struct GridBundle {
    pub name: Name,
    pub transform: Transform,
    pub visible: Visible,
    pub grid: Grid,  // Marker
    pub gpu_mesh: GpuMesh,
    pub pipeline_ref: PipelineRef,
}
```

### Query Pattern

Use query functions instead of storing entity IDs:

```rust
// Query by marker component
pub fn query_grid(world: &World) -> Option<Entity>;
pub fn query_light_gizmo(world: &World) -> Option<Entity>;
pub fn query_selectable_entities(world: &World) -> Vec<Entity>;

// Usage
if let Some(grid_entity) = query_grid(&world) {
    let transform = world.get_component::<Transform>(grid_entity);
}
```

### RefCell-Based Interior Mutability

Resources use `RefCell` for interior mutability with wrapper types:

```rust
// Access patterns
let camera = app.resource::<Camera>();           // ResRef<Camera> (immutable)
let mut camera = app.resource_mut::<Camera>();   // ResMut<Camera> (mutable)

// ResRef/ResMut auto-deref to inner type
camera.position.x += 1.0;  // Direct field access through DerefMut
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

## Architecture

### Core Modules

**`vulkanr/`** - Vulkan rendering abstraction layer
- `vulkan.rs` - Core Vulkan utilities and memory management
- `device.rs` - `RRDevice` wraps physical/logical device selection
- `swapchain.rs` - `RRSwapchain` manages the swapchain lifecycle
- `pipeline.rs` - `RRPipeline` handles graphics pipeline creation
- `render.rs` - `RRRender` manages render passes and framebuffers
- `command.rs` - `RRCommandPool` and `RRCommandBuffer` for command recording
- `buffer.rs` - Buffer abstractions (`RRVertexBuffer`, `RRIndexBuffer`, `RRUniformBuffer`)
- `descriptor.rs` - `RRDescriptorSet` for descriptor set management
- `data.rs` - `RRData` aggregates vertex/index buffers, textures, and uniforms per drawable
- `image.rs` - `RRImage` for texture image handling

**`gltf/`** - glTF model loader
- Loads meshes, textures, animations (morph targets and skeletal)
- `GltfModel` contains `GltfData` per mesh with vertex/index data
- Supports morph target animations and skeletal animations with joints

**`fbx/`** - FBX model loader (in development)
- Uses `fbxcel` and `fbxcel-dom` crates
- `FbxModel` contains `FbxData` with positions and indices
- Currently extracts mesh geometry (positions, indices)

**`math/`** - Math utilities
- Vector/matrix operations using cgmath
- Rodrigues rotation, view matrix calculation

**`support/`** - ImGui integration
- Dual-window system (ImGui debug window + Vulkan render window)
- Event handling for mouse/keyboard input
- `GUIData` struct passes input state to rendering system

**`logger/`** - Logging utilities

### Application Structure

The `App` struct in `main.rs` contains:
- Vulkan instance, device, and swapchain
- Multiple pipelines (model pipeline, grid pipeline)
- Descriptor sets per pipeline
- Command buffers for rendering
- Camera state and mouse interaction handling

**Rendering flow:**
1. Wait for previous frame fence
2. Acquire swapchain image
3. Update uniform buffers (camera transforms)
4. Update vertex buffers (for morph animations)
5. Submit command buffer
6. Present to swapchain

**Camera controls:**
- Left mouse drag: Rotate camera (updates `camera_direction` and `camera_up`)
- Middle mouse drag: Pan camera (translates `camera_pos`)
- Mouse wheel: Zoom in/out (moves camera along view direction)

### Important Constants

- `MAX_FRAMES_IN_FLIGHT = 2` - Number of concurrent GPU frames
- `VALIDATION_ENABLED` - Enabled in debug builds for Vulkan validation layers
- Validation layer: `VK_LAYER_KHRONOS_validation`

### Model Loading

**glTF models** are loaded from `assets/models/` (e.g., `stickman/stickman.glb`)
- Each mesh becomes an `RRData` with vertex/index buffers and textures
- Morph animations are stored in `GltfModel.morph_animations`

**FBX models** are loaded from `assets/models/` (e.g., `stickman/stickman_bin.fbx`)
- Currently replaces the first `RRData` vertex/index buffers
- Uses triangulation for quad faces

### Shader Pipeline

Two pipelines are created:
1. **Model pipeline** - Triangle list, fill mode, renders 3D models
2. **Grid pipeline** - Line list, line mode, renders coordinate grid

Each pipeline has its own descriptor sets for uniform buffers and textures.

## Animation System

### Animation Types

This project supports three types of animation:

1. **Skeletal Animation**
   - Deforms mesh using skin weights
   - Applied to meshes with `skin_data`
   - Calculates vertex positions using bone transform matrices and weights

2. **Node Animation**
   - Applies node hierarchy transforms without skin weights
   - Entire mesh moves/rotates according to node transforms
   - Applied to meshes WITHOUT `skin_data`

3. **Morph Target Animation**
   - Uses vertex blend shapes
   - Used for facial animations, etc.

### Related Files

| File | Role |
|------|------|
| `src/loader/gltf/loader.rs` | Load glTF files, parse node hierarchy and animation channels |
| `src/scene/animation.rs` | Define `Skeleton`, `Bone`, `AnimationClip`, `AnimationChannel` |
| `src/scene/render_resource.rs` | Execute animation updates in `RenderResources` |
| `src/app/scene_model.rs` | Set up resources when loading models |
| `src/app/update.rs` | Call animation updates per frame |

### Transform Calculation Basics

```
global_transform = parent_global_transform × local_transform
final_vertex_position = global_transform × local_vertex_position × scale_factor
```

**Key Concepts:**
- `local_transform`: Transform matrix relative to parent node
- `global_transform`: Cumulative transform matrix from root
- `base_vertices` / `local_vertices`: Original vertex positions in node-local coordinate space

### Node Animation Processing Flow

1. `AnimationClip::sample()` updates bone local transforms
2. `compute_node_global_transforms()` copies bone transforms to nodes, computes global transforms
3. `update_node_animation()` applies global transforms to each mesh's vertices

### Checklist When Modifying Animation

1. **Scale Factor Consistency**
   - Is the same scaling applied at load-time and runtime?
   - `(transform × vertex) × scale` produces DIFFERENT results than `transform × (vertex × scale)`
   - Translation component is NOT scaled in the latter case

2. **Coordinate System**
   - glTF uses Y-up, right-handed coordinate system
   - Check scale settings when exporting from Blender

3. **Node Hierarchy Verification**
   - Is the mesh attached to the correct node?
   - Are parent-child relationships parsed correctly?

4. **Rest Pose Handling**
   - When animation channels are missing, maintain current bone transform
   - Use values decomposed from original local_transform, NOT default values (0,0,0)

### Past Issues and Solutions

#### Scale Factor Mismatch Issue (2026-01)

**Symptoms**: Mesh scatters/explodes during node animation

**Root Cause**:
- Model with Armature node having 100x scale (exported from Blender)
- Loader applied 0.01 scale to `local_vertices`
- Runtime transform calculation didn't apply the same scale, causing position mismatch

**Details**:
```
Load-time: (cumulative_transform × raw_vertex) × 0.01
  → Both scale and translation are multiplied by 0.01

Runtime: global_transform × (local_vertex × 0.01)
  → Scale is correct, but translation component is NOT multiplied by 0.01
```

**Solution**:
1. `loader.rs`: Clone `local_vertices` without scaling
2. `render_resource.rs`: Add `node_animation_scale` field
3. `update_node_animation()`: Apply scale AFTER transform
   ```rust
   let pos = transform * Vector4::new(base.pos.x, base.pos.y, base.pos.z, 1.0);
   v.pos.x = pos.x * scale;
   v.pos.y = pos.y * scale;
   v.pos.z = pos.z * scale;
   ```

#### Rest Pose Value Missing Issue

**Symptoms**: Some bones snap to origin during animation playback

**Root Cause**: Default values (0,0,0) used for bones without animation channels

**Solution**: Use `decompose_transform()` to extract TRS values from current local_transform as defaults

### Debugging Tips

```rust
// Log node hierarchy and transforms
crate::log!(
    "node[{}] '{}' parent={:?} local_t=[{:.2},{:.2},{:.2}]",
    node.index, node.name, node.parent_index,
    node.local_transform[3][0], node.local_transform[3][1], node.local_transform[3][2]
);

// Compare vertices before and after transform
crate::log!(
    "base_v[0]=({:.2},{:.2},{:.2}) → after=({:.2},{:.2},{:.2})",
    base.pos.x, base.pos.y, base.pos.z,
    v.pos.x, v.pos.y, v.pos.z
);
```

### Transform Matrix Decomposition (Extract TRS)

```rust
fn decompose_transform(m: &Matrix4<f32>) -> (Vector3<f32>, Quaternion<f32>, Vector3<f32>) {
    // Translation: from 4th column
    let translation = Vector3::new(m[3][0], m[3][1], m[3][2]);

    // Scale: length of each axis vector
    let sx = (m[0][0]*m[0][0] + m[0][1]*m[0][1] + m[0][2]*m[0][2]).sqrt();
    let sy = (m[1][0]*m[1][0] + m[1][1]*m[1][1] + m[1][2]*m[1][2]).sqrt();
    let sz = (m[2][0]*m[2][0] + m[2][1]*m[2][1] + m[2][2]*m[2][2]).sqrt();
    let scale = Vector3::new(sx, sy, sz);

    // Rotation: create quaternion from rotation matrix with scale removed
    let rot_matrix = Matrix3::new(
        m[0][0]/sx, m[0][1]/sx, m[0][2]/sx,
        m[1][0]/sy, m[1][1]/sy, m[1][2]/sy,
        m[2][0]/sz, m[2][1]/sz, m[2][2]/sz,
    );
    let rotation = Quaternion::from(rot_matrix);

    (translation, rotation, scale)
}
```

### Transform Matrix Composition (TRS → Matrix4)

```rust
fn compose_transform(t: Vector3<f32>, r: Quaternion<f32>, s: Vector3<f32>) -> Matrix4<f32> {
    let rotation_matrix = Matrix4::from(r);
    let scale_matrix = Matrix4::from_nonuniform_scale(s.x, s.y, s.z);
    let translation_matrix = Matrix4::from_translation(t);
    translation_matrix * rotation_matrix * scale_matrix
}
```

### Reference Links

- [glTF 2.0 Specification - Animation](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#animations)
- [glTF 2.0 Specification - Skins](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins)
- [Bevy Engine - Animation](https://github.com/bevyengine/bevy/tree/main/crates/bevy_animation)
- [cgmath crate documentation](https://docs.rs/cgmath/latest/cgmath/)

## Development Notes

### Adding New Models

To load a new model, modify `App::load_model()`:
- For glTF: Update `model_path` variable
- For FBX: Update `model_path_fbx` variable
- Ensure textures are in the same directory or adjust texture paths

### Shader Modifications

After editing shaders in `shaders/`, the build system automatically compiles them to `assets/shaders/` directory during `cargo build`. The application loads compiled shaders from `assets/shaders/` directory.

### Camera Debugging

The ImGui debug window shows:
- Mouse position
- Click states (left/wheel)
- Current file path (for drag-and-drop)
- Reset camera buttons for debugging

Use `reset camera` to return to initial position, `reset camera up` to align camera up vector.

### Common Issues

- **Vulkan validation errors**: Check `RUST_LOG=debug` output for details
- **Shader compilation errors**: Ensure VulkanSDK is installed and `glslc` is in PATH
- **Missing textures**: Verify texture paths in model files match `assets/models/` or `assets/textures/` structure
- **FBX loading errors**: Check that FBX file is binary format (not ASCII)

### Module Organization

- Embed imgui crates locally in `src/imgui*/` for custom modifications
- Vulkan abstraction (`RR*` structs) provides a higher-level API over raw Vulkan
- Model loaders are isolated in `gltf/` and `fbx/` modules

## Testing

The project includes integration tests in the `tests/` directory:

### Test Files

**`integration_tests.rs`** - Project structure and configuration tests
- Verifies required directories exist
- Checks Cargo files and configuration
- Validates font and vendor directory structure

**`model_loading_tests.rs`** - Model loader tests
- Tests glTF and FBX model file existence
- Verifies model files are not empty
- Checks texture file availability
- Validates model directory structure

**`shader_tests.rs`** - Shader compilation tests
- Verifies shader source files exist
- Checks compiled shader files (`.spv`)
- Validates SPIR-V header format
- Ensures shader count matches between source and compiled files

### Running Tests

All tests can be run with `cargo test`. Tests verify:
- File and directory structure integrity
- Asset availability (models, textures, fonts)
- Shader compilation success
- Project configuration correctness

**Test counts**:
- Unit tests: 58 (math: 35, gltf: 11, fbx: 12)
- Integration tests: 31 (project structure: 12, model: 9, shader: 10)

**Build + Test**:
Use `build-with-tests.ps1` to run build and tests sequentially, saving results to `log/log_test.txt`

## Reference Documentation

The `memo.txt` file contains useful reference links for:
- Vulkan coordinate systems and layout qualifiers
- glTF mesh loading examples
- FBX property access patterns
- Animation and skinning techniques

## Important Rules

- Always respond in Japanese
- Do NOT perform git operations (commit, push, etc.)

## Logging

- Use the `log!` macro for logging
- Logs are output to `log/log_N.txt`
- Do NOT output to standard console; use the log files instead

## app/update.rs

- Contains the update processing
- Do NOT write feature-specific processing here
- Instead, write update processing in related files and call them from update

## mod.rs
- Don't write definition or implementation in mod.rs file
- Only module publish responsibility on mod.rs