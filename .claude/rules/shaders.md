---
paths:
  - "shaders/**"
  - "assets/shaders/**"
  - "build.rs"
---

# Shader System

## Compiling Shaders

Shaders are automatically compiled during `cargo build`. The build system compiles all shader files from `shaders/`
directory to `assets/shaders/` using glslc from VulkanSDK.

## Shader Source Files

Shader source files are located in `shaders/`:

- `vertex.vert` -> `assets/shaders/vert.spv`
- `fragment.frag` -> `assets/shaders/frag.spv`
- `gbufferVertex.vert` -> `assets/shaders/gbufferVert.spv`
- `gbufferFragment.frag` -> `assets/shaders/gbufferFrag.spv`
- `rayQueryShadow.comp` -> `assets/shaders/rayQueryShadow.spv`
- etc.

## Shader Modifications

After editing shaders in `shaders/`, the build system automatically compiles them to `assets/shaders/` directory during
`cargo build`. The application loads compiled shaders from `assets/shaders/` directory.

## Reference Documentation

The `memo.txt` file contains useful reference links for:

- Vulkan coordinate systems and layout qualifiers
- glTF mesh loading examples
- FBX property access patterns
- Animation and skinning techniques
