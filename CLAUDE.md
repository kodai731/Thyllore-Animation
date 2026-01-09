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

**通常のビルド**:
```bash
cargo build
```

**ビルド＋テスト実行（推奨）**:
```powershell
# ビルドとテストを順次実行し、結果を log/log_test.txt に保存
.\build-with-tests.ps1

# リリースビルド
.\build-with-tests.ps1 -Release

# テストをスキップ
.\build-with-tests.ps1 -SkipTests
```

このスクリプトは:
1. プロジェクトをビルド
2. ビルド成功後、全テストを実行
3. テスト結果を `log/log_test.txt` に保存

### Running the Application
```bash
# With debug logging (recommended for development)
$env:RUST_LOG="debug"; cargo run --bin rust-rendering

# Without logging
cargo run --bin rust-rendering
```

**Note**: `cargo run` 実行時は自動テストはスキップされ、アプリケーションが正常に起動します。

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

**テスト数**:
- ユニットテスト: 58個 (math: 35, gltf: 11, fbx: 12)
- インテグレーションテスト: 31個 (プロジェクト構造: 12, モデル: 9, シェーダー: 10)

**ビルド+テスト実行**:
`build-with-tests.ps1` を使用すると、ビルドとテストを順次実行し、結果を `log/log_test.txt` に保存します

## Reference Documentation

The `memo.txt` file contains useful reference links for:
- Vulkan coordinate systems and layout qualifiers
- glTF mesh loading examples
- FBX property access patterns
- Animation and skinning techniques

## 重要
- 回答は日本語で行ってください
- commitやpushなど、gitの操作は行わないでください

## log
- ログは log! マクロで記録します
- log/log_N.txt に出力されます
- 標準コンソールには出力せず、上記のファイルに記録するようにしてください

## app/update.rs
- update の処理がかかれてあります
- 各機能の処理はここに記述せず、関連する別ファイルに更新処理を書いて、それをupdateで呼ぶ形にしてください