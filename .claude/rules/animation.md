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
| `crates/thyllore-model-core/src/skinning.rs`      | `apply_skinning`, `SkinningDiagnostics` (CPU skinning vertex loop)       |
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

## Spring bones and collider fit run on the CPU

- Spring integration and collision (`crates/thyllore-anim-core/src/spring_bone/solver.rs`, `collision.rs`) stay on
  the CPU. A chain is solved joint by joint (integrate → length constraint → collision → parent rotation), so it
  parallelises only per chain; a model has about 10^2 chains, too few threads for a GPU dispatch to pay off alone.
- Move spring to a compute shader only together with GPU skinning (#265): once skin vertices and bone matrices live on
  the GPU, one dispatch (one chain per thread, joints in order, colliders in shared memory) removes the CPU spring
  loop and the vertex write-back in the same command buffer. Without GPU skinning the result has to be read back,
  which costs more than the solve.
- Collider fit (`spring_bone/collider_fit.rs` `fit_bone_capsule`, avatar-core `collider/systems/head_capsule.rs`
  `estimate_head_capsule`) runs once per user action and stays on
  the CPU. If a model makes it slow, first keep each fitted bone's vertex set from load time instead of scanning every
  vertex per fit.
- Before switching any of these to the GPU, compare tail trajectories of both implementations on the same model
  (`compare_tail_trajectories` in `spring_bone/trajectory.rs`, driven like `tools/spring_parity.py`).

## Bones are not entities

Bones are `Skeleton.bones: Vec<Bone>` identified by `BoneId`; see `ecs-architecture.md`. The editable
(keyframe) domain is `crates/thyllore-anim-core/src/editable/` with the `components/` + `systems/` split.

## Reference Links

- [glTF 2.0 Specification - Animation](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#animations)
- [glTF 2.0 Specification - Skins](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins)
- [Bevy Engine - Animation](https://github.com/bevyengine/bevy/tree/main/crates/bevy_animation)
- [cgmath crate documentation](https://docs.rs/cgmath/latest/cgmath/)
