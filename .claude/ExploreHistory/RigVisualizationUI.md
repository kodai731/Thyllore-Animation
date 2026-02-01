# Rig Visualization UI Design

## Summary

Investigated how animation software and engines visualize rigs/skeletons in the viewport. Blender's octahedral
(dual-pyramid) bone is the most established approach for animation editors. This project already has a Gizmo pipeline
(LINE_LIST, no depth test) that can be extended for bone rendering. Recommended approach: start with octahedral solid
bones using the existing gizmo pipeline pattern, rendered as an overlay with "In Front" mode.

---

## Bone Display Types Across Software

### Blender (Primary Reference)

5 bone display types available per-armature:

**Octahedral (default, recommended for this project)**
- 6 vertices, 8 triangular faces forming a dual-pyramid (diamond shape)
- Root vertex at bone head, tip vertex at bone tail, 4 mid-ring vertices forming a square cross-section
- Geometry:
  ```
  Vertices (6):
    [0] Root:  (0, 0, 0)          -- bone head
    [1] Right: (+w, d, 0)         -- mid-ring
    [2] Front: (0, d, +w)         -- mid-ring
    [3] Left:  (-w, d, 0)         -- mid-ring
    [4] Back:  (0, d, -w)         -- mid-ring
    [5] Tip:   (0, 1, 0)          -- bone tail
    w ≈ 0.1 (width), d ≈ 0.1 (depth along bone axis)

  Triangles (8):
    Lower: (0,1,2), (0,2,3), (0,3,4), (0,4,1)
    Upper: (5,2,1), (5,3,2), (5,4,3), (5,1,4)

  Wire edges (12):
    Root→mid: (0,1), (0,2), (0,3), (0,4)
    Tip→mid:  (5,1), (5,2), (5,3), (5,4)
    Ring:     (1,2), (2,3), (3,4), (4,1)
  ```
- Conveys: bone length, direction, roll (mid-ring rotation)
- Rendered as solid triangles + wireframe outline overlay

**Stick**
- Simplest: constant-width 2D line + joint dot
- No size/roll information
- Good for dense rigs with many bones

**B-Bone**
- Box shape with subdivisions showing Bezier curve deformation
- Curve visible in pose mode only

**Envelope**
- Extruded sphere showing deformation influence radius
- Radius editable directly in viewport

**Wire**
- Thin wireframe line only
- Always wireframe regardless of shading mode

### Blender Rendering Pipeline

Source: `source/blender/draw/engines/overlay/overlay_armature.cc`

Depth states per mode:
- Solid bones: `DEPTH_LESS_EQUAL` + `WRITE_DEPTH`
- Transparent bones: `DEPTH_LESS_EQUAL` + `BLEND_ADD` (no depth write)
- Selection overlay: `DEPTH_EQUAL` + `BLEND_ALPHA`
- "In Front" mode: disables depth test, bones always rendered on top of mesh

Color coding:
- 20-palette bone colors (each with normal, selected, active variants)
- Per-bone color assignment
- Custom shapes: any mesh object can be used as bone shape

### Maya

- Joints displayed as cross-shaped markers (no renderable geometry)
- Bones are visual connections between joints (viewport-only)
- "X-Ray Joints": renders joints on top of shaded objects (like Blender's "In Front")
- Orthogonal axes showing coordinate system (Z-axis blue = primary rotation axis)
- Custom visualization via plugins

### MotionBuilder

- HumanIK schematic: rectangular node display for bone mapping
- Selected effectors shown with blue outline
- FK/IK toggle per body part
- Status feedback via color (yellow = issue)

### 3ds Max

- Bone objects with taper and fin parameters for shape control
- Fins show rotation direction (side, front, back fins)
- Biped system: integrated character studio with predefined skeleton

### Unreal Engine

- `ShowDebug Bones`: white line wireframe via console command
- Animation editor: tapered diamond shapes from parent to child
- `FSkinnedSceneProxy::DebugDrawSkeleton()` uses `PDI->DrawLine()`
- Source: `Engine/Source/Runtime/Engine/Private/SkeletalDebugRendering.cpp`

### Unity

- No built-in bone renderer in core
- Animation Rigging package `BoneRenderer`: configurable boneColor, boneShape, boneSize
- Scene view only (not in Game view)
- `drawTripods` option for orientation display
- Rig Effectors: IK gizmos with customizable size/shape/color

### Godot

- Skeleton3D node with editor gizmo (improved in 4.x)
- Joint markers as circles, transform gizmo on selection
- SubGizmo implementation for unified manipulation
- PoseMode and RestMode support

### Bevy (Rust)

- No built-in bone/skeleton debug gizmo
- `bevy_gizmos` crate for immediate-mode line/shape drawing
- Typical approach: iterate skeleton hierarchy, draw lines/spheres via Gizmos API

---

## Technical Implementation

### Bone Shape Rendering Methods

**Instance Rendering (GPU, recommended for this project)**
- Single draw call renders all bones of the same shape
- Instance data per bone: transform matrix, color, length
- Vertex shader scales bone shape by length, transforms by instance matrix
- Efficient for hundreds of bones

**CPU Vertex Generation**
- Generate vertex data per frame on CPU
- Upload to dynamic vertex buffer
- Simpler implementation, adequate for <1000 bones
- Already used in this project for grid and gizmos

### Overlay Rendering Approaches

| Approach | Depth Test | Depth Write | Use Case |
|----------|-----------|-------------|----------|
| Always In Front | OFF | OFF | "In Front" mode (Blender default for armatures) |
| Depth-Aware | ON (scene depth) | OFF | Bones occluded by mesh |
| Two-Pass | Both | - | Visible = solid, occluded = transparent |

### Bone Selection / Picking

**Recommended: CPU Ray Casting (primary)**

This project already has the infrastructure for CPU ray casting in `src/math/coordinate_system.rs`:
- `screen_to_world_ray()` (line 99): mouse position to world-space ray
- `ray_to_point_distance()` (line 133): ray-to-joint distance
- `ray_to_line_segment_distance()` (line 144): ray-to-bone-segment distance
- LightGizmo selection uses this exact approach (`src/ecs/systems/gizmo_systems.rs:140`)

For octahedral bones, add `ray_to_triangle_intersection()` to test against the 8 triangular faces.
Bone count is typically tens to hundreds, so CPU intersection is fast enough without GPU involvement.

**Supplementary: G-Buffer Object ID (for selection outline)**

The project has an existing G-Buffer Object ID system (`R32_UINT` attachment) used for mesh selection
outlines. By rendering bones into the G-Buffer pass with unique Object IDs, the existing selection
outline rendering (`compositeFragment.frag:48-67` edge detection) automatically applies to bones
without additional implementation. This provides visual feedback (orange outline) for selected bones.

Approach:
1. CPU ray casting determines which bone is selected (primary picking)
2. Selected bone's Object ID is written to the selection UBO (up to 32 selected objects)
3. Composite pass renders selection outline around the bone automatically

**Not recommended: Ray Query (TLAS)**

The project's ray query pipeline (`GL_EXT_ray_query` compute shader) is used for shadow rendering.
The TLAS contains only mesh geometry, not bone gizmos. Using it for bone picking would require:
- Adding bone geometry to TLAS or building a separate TLAS
- GPU→CPU readback with latency
- Additional acceleration structure management complexity

This is overkill for bone picking and adds unnecessary complexity.

### IK Chain and Constraint Visualization

- IK chain: differently colored lines along chain
- Effector target: gizmo shape (sphere, cross, diamond) at target position
- Pole target: line from chain midpoint to pole target
- Rotation limits: arc/cone gizmo showing constraint range

### Hierarchy Lines

- Dashed/dotted lines connecting parent-child joints
- `LINE_LIST` topology
- Color matching bone theme or neutral gray

---

## Current Project Rendering Infrastructure

### Existing Gizmo Pipeline

The project already has a gizmo rendering pipeline that can be extended for bone visualization:

**Gizmo Pipeline** (`src/app/init/instance.rs:215-229`):
- Topology: `LINE_LIST`
- Polygon Mode: `LINE`
- Vertex Input: `ColorVertex` (position[3] + color[3], 24 bytes stride)
- Depth Test: **DISABLED** (always renders on top)
- Dynamic line width
- Cull mode: NONE

**Existing Gizmo Types**:
- `GridGizmoData`: Corner orientation indicator (screen-space)
- `LightGizmoData`: Selectable, draggable, distance-scaled gizmo with line mesh

**Key Data Structures**:
- `ColorVertex { pos: [f32; 3], color: [f32; 3] }` (`src/ecs/component/gizmo.rs:4-8`)
- `LineMesh = DynamicMesh<ColorVertex>` (`src/ecs/component/mesh/mod.rs:79`)
- `GizmoSelectable { is_selected, selected_axis }` (`src/ecs/component/gizmo.rs:33-37`)
- `GizmoDraggable { drag_axis, just_selected, initial_position }` (`src/ecs/component/gizmo.rs:39-54`)

### Existing Rendering Flow

`record_3d_rendering()` (`src/app/render.rs:578-629`):
```
1. Viewport & Scissor setup
2. Bind frame descriptor set (camera/light)
3. Grid mesh
4. Grid gizmo (corner orientation)
5. Light gizmo (selectable, distance-scaled)
6. Billboard (light source quad)
7. 3D models
```

Bone rendering would be inserted between grid gizmo and 3D models.

### Available Pipelines for Extension

| Pipeline | Topology | Depth | Use |
|----------|----------|-------|-----|
| Model | TRIANGLE_LIST, FILL | Enabled | 3D meshes |
| Grid | LINE_LIST, LINE | Enabled (no write) | World grid |
| Gizmo | LINE_LIST, LINE | **Disabled** | Overlays |
| Billboard | TRIANGLE_LIST, FILL | Enabled + alpha blend | Light icons |

For octahedral solid bones, a new pipeline is needed:
- Topology: `TRIANGLE_LIST`
- Polygon Mode: `FILL`
- Depth Test: Configurable (OFF for "In Front", ON for depth-aware)
- Depth Write: OFF (overlay, don't affect scene depth)
- Blend: Optional transparency for occluded bones

---

## Application Strategy for This Project

The unified implementation plan (integrating rig visualization and constraint system) is maintained in
**FbxRigAndConstraints.md** → "Unified Implementation Plan" section. Below are visualization-specific
technical details referenced by that plan.

### Recommended Bone Display: Octahedral (Blender-style)

Rationale:
- Most information density: shows direction, length, roll
- Standard in animation editing tools
- Simple geometry (6 vertices, 8 triangles) suitable for instancing
- Wireframe outline provides clear silhouette

### ECS Architecture

Following project's strict ECS patterns. Key constraint: **bones are NOT entities**.
Bones are data within `Skeleton.bones: Vec<Bone>`, identified by `BoneId` (u32 index).

**Component on model entity** (`ecs/component/`):
- `BoneGizmoState`: Per-model bone visualization state. Contains data for all bones:
  - `display_style: BoneDisplayStyle` (Octahedral, Stick, Wire) per model
  - `selected_bone: Option<BoneId>` current selection
  - `bone_colors: Vec<BoneColorState>` per-bone color (normal, selected, active)
  - `visible: bool` whether bones are shown
  - `in_front: bool` whether bones render on top

**Resource** (`ecs/resource/`):
- `BoneDisplayConfig`: Global display settings (default style, default color palette, size multiplier)

**Systems** (`ecs/systems/`):
- `build_bone_gizmo_meshes()`: Generate bone shape vertices from skeleton data
- `update_bone_gizmo_transforms()`: Update bone transforms from animation pose each frame
- `select_bone_by_ray()`: Ray cast against bone shapes for picking
- `highlight_selected_bones()`: Update bone colors based on selection state

**Entity construction**: Use existing EntityBuilder pattern (`.with_bone_gizmo_state()`),
not bundles. This project does not use a `ecs/bundle/` directory.

### Vulkan Pipeline Configuration for Solid Bones

```
New Pipeline: "BoneSolid"
  Topology:     TRIANGLE_LIST
  Polygon Mode: FILL
  Depth Test:   OFF (In Front) or ON with scene depth read
  Depth Write:  OFF
  Cull Mode:    BACK
  Blend:        OFF (solid pass) or ALPHA (transparent occluded pass)
  Dynamic:      VIEWPORT, SCISSOR
  Vertex Input: BoneVertex { pos: [f32;3], normal: [f32;3], color: [f32;3] }
  Descriptors:  Set 0 (frame UBO: view/proj), Set 2 (per-object model matrix)

New Pipeline: "BoneWire" (outline pass)
  Topology:     LINE_LIST
  Polygon Mode: LINE
  Depth Test:   OFF
  Depth Write:  OFF
  Dynamic:      VIEWPORT, SCISSOR, LINE_WIDTH
  Vertex Input: ColorVertex { pos: [f32;3], color: [f32;3] }
```

### Bone Transform Calculation

Each bone's model matrix is computed from skeleton pose data:
1. Get bone's global transform from `compute_pose_global_transforms()`
2. Scale octahedral shape along bone axis by bone length
3. Apply bone roll rotation to mid-ring vertices
4. Combine: `model_matrix = global_transform * scale_by_length * roll_rotation`

### References

- [Blender Viewport Display](https://docs.blender.org/manual/en/latest/animation/armatures/properties/display.html)
- [Blender Armature Drawing Source](https://projects.blender.org/blender/blender/src/branch/main/source/blender/draw/engines/overlay/overlay_armature.cc)
- [Blender Bone Geometry Source](https://projects.blender.org/blender/blender/src/branch/main/source/blender/draw/intern/draw_cache_impl_bone.cc)
- [Blender Overlay-Next Refactor](https://projects.blender.org/blender/blender/issues/102179)
- [Maya Joints and Bones](https://help.autodesk.com/view/MAYAUL/2025/ENU/?guid=GUID-1B59334F-2605-44C3-B584-A55B239A2CBE)
- [UE DebugDrawSkeleton](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/FSkinnedSceneProxy/DebugDrawSkeleton)
- [Unity BoneRenderer](https://docs.unity3d.com/Packages/com.unity.animation.rigging@1.1/api/UnityEngine.Animations.Rigging.BoneRenderer.html)
- [Godot Skeleton Gizmo PR](https://github.com/godotengine/godot/pull/45699)
- [Im3d - Immediate Mode 3D Gizmos](https://github.com/john-chapman/im3d)
- [Vulkan Line Rasterization](https://docs.vulkan.org/samples/latest/samples/extensions/dynamic_line_rasterization/README.html)
- [Ray Casting for Picking](https://antongerdelan.net/opengl/raycasting.html)
