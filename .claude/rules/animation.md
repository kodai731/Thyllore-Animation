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
  - "src/app/model/**"
  - "src/app/scene_model.rs"
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
| `src/ecs/systems/skeleton_pose_systems.rs`        | `sample_clip_to_pose`, `compute_pose_global_transforms`, `apply_skinning` |
| `src/ecs/systems/animation/`                      | Per-frame pipeline: `collect` → `evaluate` → `apply` → `post_process`   |
| `src/ecs/systems/phases/animation_phase.rs`       | `run_animation_phase_ecs` / `run_animation_phase_gpu` (called from `run_frame`) |
| `src/app/model/`, `src/app/scene_model.rs`        | Load a model into `AssetStorage` and GPU meshes                         |

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
- [cgmath crate documentation](https://docs.rs/cgmath/latest/cgmath/)
