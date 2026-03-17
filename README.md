# Thyllore Animation

A real-time animation and rendering engine powered by ECS architecture, built in Rust with Vulkan.

Thyllore Animation supports glTF/FBX model loading, skeletal animation, node animation, morph targets, and ML-assisted animation editing via an integrated curve copilot.

<!-- TODO: Add screenshot or GIF here -->
<!-- ![Screenshot](docs/images/screenshot.png) -->

## Features

- **Vulkan Rendering** - Deferred rendering pipeline with tone mapping and depth compositing
- **3D Model Loading** - glTF and FBX format support via `gltf` and `ufbx` crates
- **Skeletal Animation** - Bone-driven animation with hierarchical transforms
- **Node Animation** - Transform-level animation for scene objects
- **Morph Targets** - Blend shape animation support
- **ML Curve Copilot** - ONNX-based machine learning model for animation curve suggestions
- **Timeline Editor** - Keyframe editing with Bezier curve interpolation
- **Ray Tracing** - Hardware-accelerated ray tracing via Vulkan RT extensions
- **Onion Skinning** - Ghost overlay of previous/next frames for animation reference
- **ImGui Integration** - Debug UI with docking support
- **ECS Architecture** - Data-driven Entity-Component-System design inspired by Bevy

## Roadmap

See [docs/roadmap.md](docs/roadmap.md) for planned features including rig propagation, text-to-motion, and more.

## Prerequisites

### Rust

Rust 1.70 or later is required.

```bash
# Install Rust via rustup (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Vulkan SDK

This project requires the [Vulkan SDK](https://vulkan.lunarg.com/sdk/home) for the graphics backend.

#### Windows

1. Download the installer from [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home#windows)
2. Run the installer and follow the prompts
3. Verify installation:
   ```powershell
   vulkaninfo
   ```

#### Linux (Ubuntu / Debian)

```bash
# Add LunarG package repository
wget -qO- https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo tee /etc/apt/trusted.gpg.d/lunarg.asc
sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
sudo apt update

# Install Vulkan SDK
sudo apt install vulkan-sdk

# Install additional dependencies
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
```

#### macOS

```bash
# Install via Homebrew
brew install --cask vulkan-sdk
```

Or download from [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home#mac).

### GPU Driver

A Vulkan-compatible GPU and up-to-date graphics driver are required:

- **NVIDIA**: [Driver download](https://www.nvidia.com/Download/index.aspx)
- **AMD**: [Driver download](https://www.amd.com/en/support)
- **Intel**: [Driver download](https://www.intel.com/content/www/us/en/download-center/home.html)

Verify your GPU supports Vulkan:

```bash
vulkaninfo --summary
```

## Build

```bash
# Clone the repository
git clone https://github.com/kodai731/RustRendering.git
cd RustRendering

# Build
cargo build

# Build (release)
cargo build --release
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `ml` | Yes | ML-based curve copilot (requires ONNX Runtime) |
| `text-to-motion` | No | Text-to-motion generation via gRPC |

```bash
# Build without ML feature
cargo build --no-default-features

# Build with text-to-motion
cargo build --features text-to-motion
```

## Run

```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin thyllore-animation

# Run release build
cargo run --release --bin thyllore-animation
```

## Testing

```bash
# Library tests (recommended)
cargo test --lib

# Integration tests (must disable ml feature)
cargo test --test ecs_tests --no-default-features

# All tests without ml
cargo test --no-default-features
```

> **Note**: Do NOT run `cargo test --test ecs_tests` without `--no-default-features`. The ONNX Runtime dependency causes integration test crashes. See [CLAUDE.md](CLAUDE.md) for details.

## Keyboard Shortcuts

### Global

| Shortcut | Action |
|----------|--------|
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+S | Save scene |
| S | Set bone keyframe |

### Transform Gizmo

| Shortcut | Action |
|----------|--------|
| W | Translate mode |
| E | Rotate mode |
| R | Scale mode |

### Timeline Editor

| Shortcut | Action |
|----------|--------|
| Ctrl + Mouse Wheel | Zoom in/out |
| Ctrl+C | Copy keyframes |
| Ctrl+V | Paste keyframes |
| Ctrl+Shift+V | Paste keyframes (mirrored) |
| Delete | Delete selected clip/keyframe |

### Curve Editor

| Shortcut | Action |
|----------|--------|
| Ctrl+Space | Request ML curve suggestion |
| Tab | Accept suggestion overlay |
| Escape | Dismiss suggestion overlay |
| Ctrl + Mouse Wheel | Zoom at cursor position |
| Shift + Mouse Wheel | Horizontal pan |
| Mouse Wheel | Horizontal pan |
| Ctrl+Click | Toggle keyframe selection |
| Shift+Click | Range select keyframes |

## Project Structure

```
src/
├── ecs/                 # Entity-Component-System core
│   ├── component/       # Component definitions (data)
│   ├── bundle/          # Common component combinations
│   ├── resource/        # Global dynamic state
│   ├── systems/         # System functions (behavior)
│   ├── events/          # Event definitions
│   ├── world.rs         # World container
│   └── query.rs         # Entity query functions
├── animation/           # Animation system
│   └── editable/        # Editable animation (components/ + systems/)
├── app/                 # Application initialization and main loop
├── loader/              # Asset loading (glTF, FBX)
├── renderer/            # Rendering pipeline
├── platform/            # Platform layer (windowing, UI)
├── vulkanr/             # Vulkan resource management
├── ml/                  # Machine learning integration
└── main.rs              # Entry point
```

## Dependencies

Key dependencies used in this project:

| Crate | Purpose |
|-------|---------|
| [vulkanalia](https://crates.io/crates/vulkanalia) | Vulkan API bindings |
| [winit](https://crates.io/crates/winit) | Cross-platform windowing |
| [gltf](https://crates.io/crates/gltf) | glTF model loading |
| [ufbx](https://crates.io/crates/ufbx) | FBX model loading |
| [imgui](https://github.com/imgui-rs/imgui-rs) | Immediate mode GUI |
| [cgmath](https://crates.io/crates/cgmath) | Linear algebra |
| [ort](https://crates.io/crates/ort) | ONNX Runtime (ML inference) |

## Supported Platforms

| Platform | Status |
|----------|--------|
| Windows 10/11 | Primary target |
| Linux (X11) | Supported |
| macOS (MoltenVK) | Untested |

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
