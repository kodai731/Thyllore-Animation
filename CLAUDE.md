# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based Vulkan rendering engine with support for:
- 3D model loading (glTF and FBX formats)
- Skeletal animation and morph target animation
- Real-time rendering with Vulkan API
- ImGui integration for debugging UI

The project is currently on the `feature/fbx-model` branch, working on implementing FBX model support.

## Build and Run Commands

### Building the Project
```bash
cargo build
```

### Running the Application
```bash
# With debug logging (recommended for development)
$env:RUST_LOG="debug"; cargo run --bin RustRendering

# Without logging
cargo run --bin RustRendering
```

### Compiling Shaders
Shaders must be compiled to SPIR-V before running. On Windows:
```bash
cd src/shaders
./compile.bat
```

The compile script uses glslc from VulkanSDK to compile:
- `vertex.vert` → `vert.spv`
- `fragment.frag` → `frag.spv`
- `gridVertex.vert` → `gridVert.spv`
- `gridFragment.frag` → `gridFrag.spv`

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

**glTF models** are loaded from `src/resources/` (e.g., `stickman/stickman.glb`)
- Each mesh becomes an `RRData` with vertex/index buffers and textures
- Morph animations are stored in `GltfModel.morph_animations`

**FBX models** are loaded from `src/resources/` (e.g., `stickman/stickman_bin.fbx`)
- Currently replaces the first `RRData` vertex/index buffers
- Uses triangulation for quad faces

### Shader Pipeline

Two pipelines are created:
1. **Model pipeline** - Triangle list, fill mode, renders 3D models
2. **Grid pipeline** - Line list, line mode, renders coordinate grid

Each pipeline has its own descriptor sets for uniform buffers and textures.

## Development Notes

### Adding New Models

To load a new model, modify `App::load_model()`:
- For glTF: Update `model_path` variable
- For FBX: Update `model_path_fbx` variable
- Ensure textures are in the same directory or adjust texture paths

### Shader Modifications

After editing shaders in `src/shaders/src/`, run `compile.bat` to regenerate `.spv` files. The application loads compiled shaders from `src/shaders/` directory.

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
- **Missing textures**: Verify texture paths in model files match `src/resources/` structure
- **FBX loading errors**: Check that FBX file is binary format (not ASCII)

### Module Organization

- Embed imgui crates locally in `src/imgui*/` for custom modifications
- Vulkan abstraction (`RR*` structs) provides a higher-level API over raw Vulkan
- Model loaders are isolated in `gltf/` and `fbx/` modules

## Reference Documentation

The `memo.txt` file contains useful reference links for:
- Vulkan coordinate systems and layout qualifiers
- glTF mesh loading examples
- FBX property access patterns
- Animation and skinning techniques

## 重要
- 回答は日本語で行ってください
- commitやpushなど、gitの操作は行わないでください