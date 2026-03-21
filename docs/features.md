# Features

## Import / Export

- **glTF Loading** — via `gltf` crate
- **FBX Loading** — via `ufbx` crate
- **glTF Export** — re-export edited animations to glTF
- **FBX Export** — re-export edited animations to FBX

## Animation

- **Skeletal Animation** — Bone-driven animation with hierarchical transforms
- **Node Animation** — Transform-level animation for scene objects
- **Morph Targets** — Blend shape animation support
- **Onion Skinning** — Ghost overlay of previous/next frames for animation reference

## Editing

- **Timeline Editor** — Keyframe editing with Bezier curve interpolation
- **Curve Editor** — Direct manipulation of animation curves per property
- **ML Curve Copilot** — ONNX-based machine learning model for animation curve suggestions

## Rendering

- **Vulkan Rendering** — Deferred rendering pipeline with tone mapping and depth compositing
- **Ray Tracing** — Hardware-accelerated ray tracing via Vulkan RT extensions
- **Bloom** — Multi-pass bloom post-processing
- **Depth of Field** — Camera-based DOF effect
- **Auto Exposure** — Histogram-based automatic exposure adjustment

## Architecture

- **ECS Architecture** — Data-driven Entity-Component-System design inspired by Bevy Engine
- **ImGui Integration** — Docking-enabled UI for editor panels
