---
paths:
  - "src/animation/**"
  - "src/loader/**"
  - "src/scene/**"
  - "src/app/scene_model.rs"
  - "src/ecs/systems/animation_*"
  - "src/ecs/systems/skeleton_*"
  - "src/ecs/systems/pose_*"
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

| File                           | Role                                                           |
|--------------------------------|----------------------------------------------------------------|
| `src/loader/gltf/loader.rs`    | Load glTF files, parse node hierarchy and animation channels   |
| `src/scene/animation.rs`       | Define `Skeleton`, `Bone`, `AnimationClip`, `AnimationChannel` |
| `src/scene/render_resource.rs` | Execute animation updates in `RenderResources`                 |
| `src/app/scene_model.rs`       | Set up resources when loading models                           |
| `src/app/update.rs`            | Call animation updates per frame                               |

## Transform Calculation Basics

```
global_transform = parent_global_transform * local_transform
final_vertex_position = global_transform * local_vertex_position * scale_factor
```

**Key Concepts:**

- `local_transform`: Transform matrix relative to parent node
- `global_transform`: Cumulative transform matrix from root
- `base_vertices` / `local_vertices`: Original vertex positions in node-local coordinate space

## Node Animation Processing Flow

1. `AnimationClip::sample()` updates bone local transforms
2. `compute_node_global_transforms()` copies bone transforms to nodes, computes global transforms
3. `update_node_animation()` applies global transforms to each mesh's vertices

## Checklist When Modifying Animation

1. **Scale Factor Consistency**
    - Is the same scaling applied at load-time and runtime?
    - `(transform * vertex) * scale` produces DIFFERENT results than `transform * (vertex * scale)`
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

## Past Issues and Solutions

### Scale Factor Mismatch Issue (2026-01)

**Symptoms**: Mesh scatters/explodes during node animation

**Root Cause**:

- Model with Armature node having 100x scale (exported from Blender)
- Loader applied 0.01 scale to `local_vertices`
- Runtime transform calculation didn't apply the same scale, causing position mismatch

**Solution**:

1. `loader.rs`: Clone `local_vertices` without scaling
2. `render_resource.rs`: Add `node_animation_scale` field
3. `update_node_animation()`: Apply scale AFTER transform

### Rest Pose Value Missing Issue

**Symptoms**: Some bones snap to origin during animation playback

**Root Cause**: Default values (0,0,0) used for bones without animation channels

**Solution**: Use `decompose_transform()` to extract TRS values from current local_transform as defaults

## Transform Matrix Decomposition (Extract TRS)

```rust
fn decompose_transform(m: &Matrix4<f32>) -> (Vector3<f32>, Quaternion<f32>, Vector3<f32>) {
    let translation = Vector3::new(m[3][0], m[3][1], m[3][2]);

    let sx = (m[0][0] * m[0][0] + m[0][1] * m[0][1] + m[0][2] * m[0][2]).sqrt();
    let sy = (m[1][0] * m[1][0] + m[1][1] * m[1][1] + m[1][2] * m[1][2]).sqrt();
    let sz = (m[2][0] * m[2][0] + m[2][1] * m[2][1] + m[2][2] * m[2][2]).sqrt();
    let scale = Vector3::new(sx, sy, sz);

    let rot_matrix = Matrix3::new(
        m[0][0] / sx, m[0][1] / sx, m[0][2] / sx,
        m[1][0] / sy, m[1][1] / sy, m[1][2] / sy,
        m[2][0] / sz, m[2][1] / sz, m[2][2] / sz,
    );
    let rotation = Quaternion::from(rot_matrix);

    (translation, rotation, scale)
}
```

## Transform Matrix Composition (TRS -> Matrix4)

```rust
fn compose_transform(t: Vector3<f32>, r: Quaternion<f32>, s: Vector3<f32>) -> Matrix4<f32> {
    let rotation_matrix = Matrix4::from(r);
    let scale_matrix = Matrix4::from_nonuniform_scale(s.x, s.y, s.z);
    let translation_matrix = Matrix4::from_translation(t);
    translation_matrix * rotation_matrix * scale_matrix
}
```

## Reference Links

- [glTF 2.0 Specification - Animation](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#animations)
- [glTF 2.0 Specification - Skins](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins)
- [Bevy Engine - Animation](https://github.com/bevyengine/bevy/tree/main/crates/bevy_animation)
- [cgmath crate documentation](https://docs.rs/cgmath/latest/cgmath/)
