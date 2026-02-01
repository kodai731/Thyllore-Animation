# Bone Gizmo Position Calculation

## Summary
Bone wireframe (rig) visualization had three layered issues: (1) wrong shader pipeline caused all bone vertex data to be discarded, (2) FBX bones had identity `inverse_bind_pose` in `global_transforms` giving all-zero positions, (3) bind-pose-only positions didn't track animation. Fixed by switching to grid pipeline, using IBP-based local offsets, and applying `current_global * local_offset` per frame.

## Date
2026-02-01

## Issue 1: Wrong Shader Pipeline (Root Cause of Invisibility)

### Symptom
Bone gizmo lines were completely invisible despite having valid vertex data, buffer handles, and pipeline ID.

### Root Cause
`BoneGizmoData` was initialized with `gizmo_pipeline_id` which uses `gizmoVertex.vert`. This shader is designed for the orientation gizmo widget (fixed screen-space position at bottom-right corner). It:
- Ignores `object.model` matrix entirely
- Uses hardcoded screen-space coordinates (`gizmoOffset = vec2(0.75, -0.75)`)
- Does NOT apply `frame.proj * frame.view * worldPos` transformation

All bone vertex positions were discarded by the shader.

### Fix
Changed pipeline assignment in `src/app/init/instance.rs`:
```rust
// Before: used gizmo pipeline (screen-space shader)
bone_gizmo_data.render_info.pipeline_id = Some(gizmo_pipeline_id);
// After: use grid pipeline (world-space shader with view/proj transform)
bone_gizmo_data.render_info.pipeline_id = Some(grid_pipeline_id);
```

`gridVertex.vert` correctly transforms vertices: `gl_Position = frame.proj * frame.view * object.model * vec4(inPosition, 1.0)`

### Key Lesson
When adding a new renderable that shares pipeline with an existing one, verify the shader actually processes vertex data as expected. The gizmo pipeline and grid pipeline both use `LINE_LIST` topology and `VertexInputConfig::Gizmo`, but their shaders have completely different vertex transformation logic.

## Issue 2: FBX Bone Positions All at Origin

### Symptom
For FBX models (fly.fbx), all bone positions computed from `global_transforms` were (0,0,0). The `global_transforms[i][3]` (translation column) had no position data.

### Root Cause
FBX bone `local_transform` values (from russimp node hierarchy) lack translation data for many models. The FBX node tree stores bone transforms as identity-like matrices, with actual bind-pose positions only available in cluster data (`inverse_bind_pose` from `bone.offset_matrix`).

When `compute_pose_global_transforms` computes `parent_global * local`, the result has no translations since locals are identity.

This is fundamentally different from glTF where `bone.local_transform` contains proper translations.

### Analysis of Approaches Tried

**Approach 1: `inv_root * global * origin`** — Works for glTF, fails for FBX (all zero).

**Approach 2: `inv_root * inverse(IBP) * origin`** — Gives correct bind-pose positions for FBX, but:
- glTF had `identity_ibp=14` (loader doesn't set IBP on bones) → all positions at origin
- Positions are static (no animation tracking)

**Approach 3: `skin_matrix * bind_display_pos`** — Mathematically cancels out: `(global * IBP) * (inverse(IBP) * origin) = global * origin` → back to zero for FBX.

### Fix
Hybrid approach with pre-computed local offsets:

```rust
// Computed once at skeleton initialization
fn compute_bone_local_offsets(skeleton, rest_global_transforms) -> Vec<[f32; 3]> {
    for each bone:
        if IBP != identity:  // FBX path
            bind_world_pos = inverse(IBP) * origin
            offset = inverse(rest_global) * bind_world_pos
        else:  // glTF path (IBP not set)
            offset = (0, 0, 0)  // use global_transform directly
}

// Applied every frame
fn compute_display_transforms(skeleton, current_globals, offsets) {
    for each bone:
        animated_world = current_globals[i] * Vector4(offset, 1.0)
        display = inv_root * animated_world
}
```

**Why this works for both formats:**
- **FBX** (IBP non-identity, rest_global ≈ Identity):
  - `offset ≈ inverse(IBP) * origin` = bind-time world position
  - `animated = current_global * offset` — at rest gives bind position, during animation applies delta
- **glTF** (IBP = identity, rest_global has bone positions):
  - `offset = (0,0,0)` → `animated = current_global * (0,0,0,1) = current_global_pos`
  - Directly uses the global transform's translation column

## Issue 3: glTF `inverse_bind_pose` Not Populated

### Symptom
Stickman model (glTF) had `identity_ibp=14` — all 14 bones had identity IBP.

### Root Cause
The glTF loader does not copy `inverse_bind_matrices` from skin data to `bone.inverse_bind_pose`. The `Bone::default()` sets `inverse_bind_pose = Matrix4::identity()`.

### Workaround
The hybrid approach handles this by falling back to `global_transforms` when IBP is identity. The offset is (0,0,0) meaning `current_global * origin` gives the bone position directly from the animation system's global transforms, which ARE correct for glTF.

## Files Changed

| File | Change |
|------|--------|
| `src/app/init/instance.rs` | `gizmo_pipeline_id` → `grid_pipeline_id` for BoneGizmoData |
| `src/debugview/gizmo/bone.rs` | Added `bone_local_offsets: Vec<[f32; 3]>` field |
| `src/ecs/systems/bone_gizmo_systems.rs` | Added `compute_bone_local_offsets()`, updated `build_bone_line_mesh` and `compute_display_transforms` to use offsets |
| `src/app/model_loader.rs` | Compute and cache offsets in `initialize_bone_gizmo_visibility` |
| `src/ecs/systems/phases/render_prep_phase.rs` | Pass offsets to `build_bone_line_mesh` |

## Phase 3: Bone Selection and Color Highlight (2026-02-02)

### Implementation

CPU raycast-based bone selection with octahedral bone triangle hit testing and color highlight.

### Architecture Decisions
- **CPU raycasting**: Sufficient accuracy for bone picking without G-Buffer Object ID integration
- **BoneSelectionState separated from BoneGizmoData**: Keeps rendering data and selection state decoupled
- **Shift+click multi-select**: Uses `gui_data.is_shift_pressed` from imgui io
- **Octahedral mode only**: Stick mode selection deferred

### Key Components
- `ray_to_triangle_intersection()` in `math/coordinate_system.rs` — Moller-Trumbore algorithm
- `BoneSelectionState` in `debugview/gizmo/bone_selection.rs` — HashSet<usize> + active_bone_index
- `compute_octahedral_vertices_per_bone()` — Shared vertex computation for hit test and rendering
- `select_bone_by_ray()` — Tests all octahedral triangles, returns closest hit
- `process_bone_selection()` in input_phase — Handles click detection, shift-toggle, selection clear
- `build_octahedral_bone_meshes_with_selection()` — Per-bone color based on selection state

### Color Scheme
| State | Solid Color | Wire Color |
|-------|-------------|------------|
| Normal | [0.2, 0.45, 0.7] | [0.05, 0.15, 0.35] |
| Selected | [0.4, 0.7, 1.0] | [0.1, 0.3, 0.55] |
| Active | [1.0, 0.6, 0.2] | [0.5, 0.3, 0.1] |

### Files Changed
| File | Change |
|------|--------|
| `src/math/coordinate_system.rs` | Added `ray_to_triangle_intersection()` + tests |
| `src/math/mod.rs` | Exported new function |
| `src/debugview/gizmo/bone_selection.rs` | **New** — `BoneSelectionState` resource |
| `src/debugview/gizmo/mod.rs` | Added module export |
| `src/app/gui_data.rs` | Added `is_shift_pressed` field |
| `src/platform/events.rs` | Added `io.key_shift` capture |
| `src/app/init/instance.rs` | Added `BoneSelectionState::default()` initialization |
| `src/ecs/systems/bone_gizmo_systems.rs` | Selection colors, vertex computation, ray selection |
| `src/ecs/systems/phases/input_phase.rs` | Added `process_bone_selection()` |
| `src/ecs/context.rs` | Added `bone_selection()` / `bone_selection_mut()` accessors |
| `src/ecs/systems/phases/render_prep_phase.rs` | Uses selection state for mesh building |

## Debugging Tips

When bone positions are wrong, check these in order:
1. **Shader pipeline** — Does the shader actually use world-space vertex transformation?
2. **IBP values** — Are `inverse_bind_pose` matrices identity or populated? Log `identity_ibp` count.
3. **Coordinate space** — `inv_root` must cancel `root_transform` to match mesh display space.
4. **Scale** — FBX `unit_scale` is applied to both mesh vertices and IBP translation components.
