---
paths:
  - "crates/thyllore-anim-core/**"
  - "crates/thyllore-model-core/**"
  - "crates/thyllore-importer-core/**"
  - "src/ecs/systems/animation/**"
  - "src/ecs/systems/animation_*"
  - "src/ecs/systems/skeleton_*"
  - "src/ecs/systems/pose_*"
  - "src/ecs/systems/phases/animation_phase.rs"
  - "src/ecs/systems/model_load_systems/**"
---

# Animation System

## Animation Types

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

## Related Files

| File                                              | Role                                                                    |
|---------------------------------------------------|-------------------------------------------------------------------------|
| `crates/thyllore-model-core/src/skeleton.rs`      | `Skeleton`, `Bone` (static rig data)                                    |
| `crates/thyllore-anim-core/src/clip.rs`           | `AnimationClip`, `TransformChannel`, `compose_transform` / `decompose_transform` |
| `crates/thyllore-anim-core/src/pose.rs`           | `SkeletonPose`, `BoneLocalPose`                                         |
| `crates/thyllore-importer-core/src/gltf/loader.rs`| Load glTF files, parse node hierarchy and animation channels            |
| `crates/thyllore-importer-core/src/model_result.rs` | Loader output incl. `node_animation_scale`                            |
| `crates/thyllore-model-core/src/skinning.rs`      | `apply_skinning`, `SkinningDiagnostics` (CPU skinning vertex loop)       |
| `crates/thyllore-model-core/src/palette_skinning.rs` | `build_skin_palette`, `pack_skin_influences`, `skin_geometry` (the palette model the device skins with, and its CPU evaluation) |
| `shaders/model/skinningCompute.slang`             | Per-frame skinning of every skinned mesh on the device                  |
| `src/ecs/systems/animation/`                      | Per-frame pipeline: `collect` → `evaluate` → `apply` → `post_process`   |
| `src/ecs/systems/phases/animation_phase.rs`       | `run_animation_phase_ecs` / `run_animation_phase_gpu` (called from `run_frame`) |
| `src/ecs/systems/model_load_systems/`           | Load a model into `AssetStorage` and GPU meshes |

## Skeletal Animation Runs on the Device

- The animation phase only poses a skinned mesh: `apply_skinning_to_single_mesh` stores the skin palette
  (`MeshBuffer::pose_skin`); it never rewrites `vertex_data.vertices`, which stay as imported.
- `VulkanBackend::stage_animated_meshes` writes the palette of this frame slot and, when the skin changed,
  stages the rest geometry and influences; `record_animated_mesh_uploads` records the stream copies, one
  `pass.skinning` dispatch per mesh and the BLAS / TLAS updates at the head of the frame's command buffer.
- A mesh has two vertex streams: geometry (position, normal; rewritten by skinning, node animation and morphs,
  read by the BLAS) and attributes (color, tex coord). `VertexGeometry` / `VertexAttributes` in model-core
  are the one definition of their layout; the hit shaders read them through `hit_shading_record.slang`.
- `skin_data` is edited only through `MeshBuffer::edit_skin_data` / `set_skin_data`, which mark the device
  streams stale so the next pose stages them again. Code that needs posed positions on the CPU (bounds,
  picking, dumps) calls `MeshBuffer::posed_geometry`, the CPU evaluation of the same palette.
- `apply_skinning` remains for CPU-only consumers: onion skin ghosts, the spring bake and tests.

## Current CPU / Device Boundary

This is the boundary as of 2026-11 (decided in #266). It describes where the work runs today and why; it is
not a permanent rule, so revisit it when one of the conditions below changes.

- Per-vertex work runs on the device: skinning today, and it is the intended place for future per-vertex
  work such as cloth or hair strands. Morph targets are still accumulated on the CPU when a weight changes
  and reach the device as new rest geometry.
- Per-bone work runs on the CPU: clip sampling, the global transform hierarchy, spring bone integration and
  collision, collider fitting, and the skin palette build. The palette is uploaded once per frame.

Why the boundary sits here today:

- Only skin vertices live on the device; the bone transforms the springs read and write are CPU data.
- The spring result has CPU consumers in the same frame (the palette build, gizmos, bake, export, the
  Blender and Unity parity dumps), so a device-side spring would need a readback every frame.
- The bone count is small (about 10^3 bones, 10^2 chains) next to the vertex count (about 10^5), so the
  device has little to gain; Unity, UniVRM, Unreal, Bevy and Maya draw the same line.

Conditions that would justify moving it:

- The spring result stops having same-frame CPU consumers, or the palette build itself moves to the device.
- The simulated element count grows to per-vertex scale (cloth, strand hair), which is then per-vertex work.
- An optimized-build measurement shows bone-level work dominating the frame.

## Transform Calculation Basics

```
global_transform = parent_global_transform * local_transform
final_vertex_position = global_transform * local_vertex_position * scale_factor
```

**Key Concepts:**

- `local_transform`: Transform matrix relative to parent node
- `global_transform`: Cumulative transform matrix from root
- `base_vertices`: Original vertex positions in node-local coordinate space (kept on the GPU mesh)

## Node Animation Processing Flow

1. `sample_clip_to_pose()` fills a `SkeletonPose` from the clip channels
2. `compute_node_global_transforms()` (`animation/apply.rs`) copies bone poses to nodes and computes global transforms
3. `prepare_node_animation()` applies each node's global transform to its mesh's `base_vertices`, then multiplies by
   `node_animation_scale`

## Checklist When Modifying Animation

1. **Scale Factor Consistency**
    - Is the same scaling applied at load-time and runtime?
    - `(transform * vertex) * scale` produces DIFFERENT results than `transform * (vertex * scale)`
    - Translation component is NOT scaled in the latter case
    - `node_animation_scale` is decided once by the importer (`model_result.rs`: 0.01 for an armature export,
      otherwise 1.0) and applied after the transform

2. **Coordinate System**
    - glTF uses Y-up, right-handed coordinate system
    - Check scale settings when exporting from Blender

3. **Node Hierarchy Verification**
    - Is the mesh attached to the correct node?
    - Are parent-child relationships parsed correctly?

4. **Rest Pose Handling**
    - When animation channels are missing, maintain current bone transform
    - Use values decomposed from original local_transform (`decompose_transform`), NOT default values (0,0,0)

## Bones are not entities

Bones are `Skeleton.bones: Vec<Bone>` identified by `BoneId`; see `ecs-architecture.md`. The editable
(keyframe) domain is `crates/thyllore-anim-core/src/editable/` with the `components/` + `systems/` split.

## Reference Links

- [glTF 2.0 Specification - Animation](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#animations)
- [glTF 2.0 Specification - Skins](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins)
- [Bevy Engine - Animation](https://github.com/bevyengine/bevy/tree/main/crates/bevy_animation)
- [glam crate documentation](https://docs.rs/glam/latest/glam/)
