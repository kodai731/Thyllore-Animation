---
paths:
  - "src/vulkanr/**"
  - "src/app/**"
  - "src/renderer/**"
  - "src/gltf/**"
  - "src/fbx/**"
  - "src/math/**"
  - "src/support/**"
---

# Architecture Details

## Core Modules

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

## Application Structure

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

## Important Constants

- `MAX_FRAMES_IN_FLIGHT = 2` - Number of concurrent GPU frames
- `VALIDATION_ENABLED` - Enabled in debug builds for Vulkan validation layers
- Validation layer: `VK_LAYER_KHRONOS_validation`

## Model Loading

**glTF models** are loaded from `assets/models/` (e.g., `stickman/stickman.glb`)

- Each mesh becomes an `RRData` with vertex/index buffers and textures
- Morph animations are stored in `GltfModel.morph_animations`

**FBX models** are loaded from `assets/models/` (e.g., `stickman/stickman_bin.fbx`)

- Currently replaces the first `RRData` vertex/index buffers
- Uses triangulation for quad faces

## Shader Pipeline

Two pipelines are created:

1. **Model pipeline** - Triangle list, fill mode, renders 3D models
2. **Grid pipeline** - Line list, line mode, renders coordinate grid

Each pipeline has its own descriptor sets for uniform buffers and textures.

## Module Organization

- Embed imgui crates locally in `src/imgui*/` for custom modifications
- Vulkan abstraction (`RR*` structs) provides a higher-level API over raw Vulkan
- Model loaders are isolated in `gltf/` and `fbx/` modules

## Camera Debugging

The ImGui debug window shows:

- Mouse position
- Click states (left/wheel)
- Current file path (for drag-and-drop)
- Reset camera buttons for debugging

Use `reset camera` to return to initial position, `reset camera up` to align camera up vector.

## Common Issues

- **Vulkan validation errors**: Check `RUST_LOG=debug` output for details
- **Shader compilation errors**: Ensure VulkanSDK is installed and `glslc` is in PATH
- **Missing textures**: Verify texture paths in model files match `assets/models/` or `assets/textures/` structure
- **FBX loading errors**: Check that FBX file is binary format (not ASCII)
