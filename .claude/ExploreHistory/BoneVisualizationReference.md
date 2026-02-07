# Bone/Skeleton Visualization Reference

Summary: Comprehensive research on how various animation software and game engines visualize
rigs/skeletons/bones in their viewports, including technical implementation details for rendering
bone gizmos in a Vulkan-based engine.

## 1. Blender Bone Visualization

### 1.1 Bone Display Types

Blender provides 5 bone display types, each with different geometry and information conveyed:

#### Octahedral (Default)
- 6-vertex, 8-face bipyramid (octahedron)
- Vertex layout:
  - Vertex 0: Root/head at (0, 0, 0)
  - Vertex 1: Tip/tail at (0, 1, 0)
  - Vertices 2-5: Square cross-section at ~0.1 along Y, offset in +X, +Z, -X, -Z
- Conveys: root/tip position, bone size (thickness proportional to length), bone roll (square section)
- Best for general editing tasks
- Drawn as solid triangles with wireframe outline

#### Stick
- Simplest visualization: constant-thickness sticks
- Rendered as 2D lines with dots at joints
- No information about root/tip differentiation, bone size, or roll angle
- Minimal visual footprint, good for dense rigs

#### B-Bone (Bendy Bone)
- Displayed as box shapes showing subdivision and B-Spline curves
- Multi-segment bones with configurable curvature
- In Edit mode, still drawn as sticks (need Pose mode to see curve)
- Useful for flexible deformation chains (spines, tails, tentacles)

#### Envelope
- Extruded sphere shapes showing deformation influence volume
- Visualizes bone Dist (distance) property for envelope deformation
- Allows editing influence radius directly in viewport
- Scaling affects envelope radius rather than bone geometry

#### Wire
- Thin wireframe lines
- Bones always displayed as wireframe regardless of viewport shading mode
- Useful for non-obstructive custom bone shapes

### 1.2 Rendering Pipeline (Blender Overlay Engine)

Source: `source/blender/draw/engines/overlay/overlay_armature.cc`
Geometry: `source/blender/draw/intern/draw_cache_impl_bone.cc`

#### Draw Passes and Depth States
- Solid bones: `DRW_STATE_WRITE_COLOR | DRW_STATE_DEPTH_LESS_EQUAL | DRW_STATE_WRITE_DEPTH`
- Transparent bones: `DRW_STATE_WRITE_COLOR | DRW_STATE_DEPTH_LESS_EQUAL | DRW_STATE_BLEND_ADD`
- Selection overlay: `DRW_STATE_WRITE_COLOR | DRW_STATE_DEPTH_EQUAL | DRW_STATE_BLEND_ALPHA`
- In-front mode: Additional `DRW_STATE_IN_FRONT_SELECT` flag

#### Overlay Architecture
- All overlays rendered by a single Overlay draw engine
- Active overlays stored as flags in the 3D view
- Blender 4.4+ rewrote the Overlay engine (Overlay-Next) to:
  - Remove globals access (enable parallel rendering for EEVEE)
  - Improve selection speed
  - Port geometry shaders to primitive expansion API
  - Reduce shader duplication for Metal backend

#### Strategy Pattern for Draw Types
- PR #106232 introduced `ArmatureBoneDrawStrategy` subclasses
- Each display type (Octahedral, Stick, BBone, etc.) has its own strategy
- Eliminates `switch (arm->drawtype)` scattered through the code

### 1.3 Selection and Interaction

- "In Front" option: bones always render on top of solid objects
- When enabled, bones are always visible and selectable
- Blender removed culling tests for armatures (selection is as fast as drawing)
- OpenGL depth picking was problematic (bones with constraints needed double-click)

### 1.4 Color Coding

- Theme defines 20 bone color palettes, each with 3 colors:
  - Normal (unselected) color
  - Selected outline color
  - Active bone outline color
- Per-bone color assignment (stored on Bone, visible in Pose + Edit modes)
- Per-armature color override (stored on PoseBone, Pose mode only)
- Color processing: sRGB internally, converted to linear via `srgb_to_linearrgb_v4()`
- New presets (Blender 4.x) sorted by Hue using OKLab luminance calculation
- Custom shapes: any mesh object as bone shape, with optional wireframe-only display

### 1.5 Custom Shapes
- Any mesh object can replace default bone shape
- Y axis aligned along bone direction
- Shape scaled so 1 unit = bone length
- Wireframe checkbox forces wireframe display regardless of shading mode


## 2. Maya Bone/Joint Visualization

### 2.1 Joint Display
- Joints are the building blocks of skeletons
- Joints have NO renderable shape (viewport-only visualization)
- Bones are visual connectors between joints in scene view
- Neither joints nor bones appear in rendered output

### 2.2 Display Features
- **X-Ray Joints mode**: displays joints over shaded objects (similar to Blender "In Front")
- Joint coordinate systems shown as orthogonal axes
- Z-axis (blue) conventionally set as primary rotation axis
- Each joint has a rotational pivot point

### 2.3 Joint Types
- Hinge joint: rotates about only one local axis
- Ball joint: full rotation freedom
- Joints created with Joint Tool, bones auto-connect between placed joints

### 2.4 Technical Notes
- Joints are special node types in Maya's DAG hierarchy
- Joint acts as parent node for child joints below it
- Custom visualization possible via plugins (e.g., boneDynamicsNode)


## 3. MotionBuilder Skeleton Visualization

### 3.1 Character Controls
- HumanIK nodes laid out in schematic biped arrangement
- Visual mapping tool for skeleton structure
- Color-coded feedback on characterization status (yellow = problems)
- Schematic view: bones as rectangular nodes for easy selection

### 3.2 Effector Display
- Character representation shows all effectors for Control Rig
- Selected effectors highlighted with blue contour
- FK/IK switching supported
- Effectors for individual body parts (wrist, ankle, etc.)

### 3.3 Bone Mapping
- Mapping List identifies bones by slot assignment
- Base group: minimum required bones for characterization
- Supports joints, bones, nulls, or any object as mapping target
- Custom bone names via Skeleton Definition Files


## 4. 3ds Max Bone Visualization

### 4.1 Bone Objects
- Bones are renderable objects (unlike Maya)
- Configurable parameters: taper and fins
- Fins: visual aids for bone orientation (side, front, back), off by default
- By default NOT renderable; must enable in Object Properties

### 4.2 Display Modes
- Standard bone geometry
- Display as links (line connections)
- BoneAsLine mode (accessible only via Python, not UI)
- Diamond/tick display modes (format-dependent on import)

### 4.3 Biped System
- Integrated Character Studio (since Max 4)
- Pre-built biped skeleton with stock settings
- IK/FK switching, pose manipulation, layers, keyframing
- Animation data sharing across different Biped skeletons

### 4.4 Technical Notes
- Any hierarchy can display as bone structure ("Bone On" in Bone Editing Tools)
- Bone geometry extends from pivot to child object
- Think of bones as joints: pivot placement matters more than geometry


## 5. Game Engines

### 5.1 Unreal Engine
- **ShowDebug Bones**: console command displays bones as white lines
- **DebugDrawSkeleton**: `FSkinnedSceneProxy::DebugDrawSkeleton()` in SkinnedMeshComponent
- **UDebugSkelMeshComponent**: specialized editor component for animation viewport
- **Skeleton Editor**: visual editor for bone manipulation
- **Observe Bone**: debug node showing transform information per bone
- Bone rendering uses PDI->DrawLine() for wireframe edges
- Pyramid/rhombus shape: 4-sided tapered diamond from parent to child
- Source: `Engine/Source/Runtime/Engine/Private/SkeletalDebugRendering.cpp`

### 5.2 Unity
- No built-in bone debug renderer out of the box
- **BoneRenderer** component (Animation Rigging package):
  - Defines transform hierarchy drawn as bones
  - Configurable: boneColor, boneShape, boneSize
  - Scene view only (not visible in Game view)
  - drawTripods option for orientation display
- **Rig Effectors**: gizmos on transforms for IK manipulation
  - Customizable: size, shape, color, offset, custom mesh
- Custom debug: `Debug.DrawLine` / `Gizmos.DrawLine` for recursive hierarchy traversal
- Two Bone IK: Target (hand) and Hint (elbow) effectors

### 5.3 Godot
- **Skeleton3D** node: improved editor visualization in 4.x
- **PR #45699**: Skeleton Editor Gizmo implementation
  - Marker circles on joints
  - Transform gizmo on selected markers (SubGizmo)
  - PoseMode and RestMode
  - Source: `skeleton_3d_editor_plugin.cpp`
- **SkeletonModifier3D**: modifier system for bone manipulation
  - Executes in `_process_modification()` virtual method
  - Processing order matches child list order
- Runtime debug: `ImmediateMesh` or `MeshInstance3D` for custom drawing
- Third-party: BoneGizmo plugin for simple bone transform editing

### 5.4 Bevy Engine (Rust)
- No built-in bone/skeleton debug gizmo in core engine
- `bevy_gizmos` crate for immediate mode drawing:
  - `Gizmos` system parameter: `gizmos.line(Vec3::ZERO, Vec3::X, GREEN)`
  - Supports lines, arcs, arrows, circles, crosses, curves, grids
  - Retained mode (`Gizmo` component) for static lines
- `bevy_animation_graph` (third-party): bone debug rendering in preview scene
- Typical approach: iterate skeleton hierarchy, draw lines/spheres via Gizmos API


## 6. Technical Implementation Details

### 6.1 Bone Shape Geometry

#### Octahedral/Pyramid (Most Common)
```
Vertices (6):
  [0] Root:  (0.0, 0.0, 0.0)     -- bone head
  [1] Right: (+w, d, 0.0)         -- side vertex
  [2] Front: (0.0, d, +w)         -- side vertex
  [3] Left:  (-w, d, 0.0)         -- side vertex
  [4] Back:  (0.0, d, -w)         -- side vertex
  [5] Tip:   (0.0, 1.0, 0.0)     -- bone tail

where w = width factor (~0.1), d = depth along bone (~0.1)

Triangles (8):
  Bottom pyramid: (0,1,2), (0,2,3), (0,3,4), (0,4,1)
  Top pyramid:    (5,2,1), (5,3,2), (5,4,3), (5,1,4)

Wire edges (8):
  (0,1), (0,2), (0,3), (0,4), (5,1), (5,2), (5,3), (5,4)
  Plus ring: (1,2), (2,3), (3,4), (4,1)
```

#### Stick (Simplest)
- Line from root to tip
- Small sphere/dot at each joint
- Constant pixel-width (screen-space)

#### Box/BBone
- Rectangular box stretched along bone axis
- Subdivisions for B-spline curvature visualization

### 6.2 Rendering Approach

#### Separate Overlay Pass (Recommended for Vulkan)
1. **Main scene pass**: Render meshes with normal depth testing/writing
2. **Overlay pass**: Render bone gizmos with modified depth behavior
   - Option A: Depth test disabled (`depthTestEnable = VK_FALSE`) for "always on top"
   - Option B: Depth test against scene buffer but no depth write for "occluded" bones
   - Option C: Two sub-passes, one for visible bones and one for occluded (rendered as transparent)

#### Blender's Approach (Reference)
- Uses DRW (Draw Manager) with multiple passes:
  - Solid pass with depth write
  - Transparent pass with additive blend
  - Selection pass with alpha blend
  - "In Front" flag for always-visible bones
- GPUBatch combines geometry + shader + parameters for draw calls

#### Depth Testing Configuration (Vulkan)
```
VkPipelineDepthStencilStateCreateInfo:
  // Normal scene rendering
  depthTestEnable  = VK_TRUE
  depthWriteEnable = VK_TRUE
  depthCompareOp   = VK_COMPARE_OP_LESS_OR_EQUAL

  // Overlay gizmos (always on top)
  depthTestEnable  = VK_FALSE
  depthWriteEnable = VK_FALSE

  // Overlay gizmos (depth-aware, no write)
  depthTestEnable  = VK_TRUE
  depthWriteEnable = VK_FALSE
  depthCompareOp   = VK_COMPARE_OP_LESS_OR_EQUAL
```

### 6.3 GPU vs CPU Rendering

#### GPU (Preferred for Performance)
- Instanced rendering: one draw call for all bones of same type
- Each bone instance: transform matrix as per-instance data
- Vertex shader: transform bone shape vertices by instance matrix
- Im3d approach: instanced triangle strips, vertex shader expansion

#### CPU (Simpler Implementation)
- Generate vertex data on CPU each frame
- Upload to dynamic vertex buffer
- Single draw call with pre-transformed vertices
- Easier to implement, suitable for moderate bone counts (<1000)

### 6.4 Bone Picking/Selection

#### Ray Casting (Most Common)
1. Unproject 2D screen click to 3D ray from camera
2. Test ray against bone bounding shapes:
   - Capsule: elongated cylinder with hemispherical caps
   - Sphere: at each joint for joint selection
   - Custom geometry: ray-triangle for octahedral shapes
3. CPU-based (only 1 ray per click, fast enough)
4. Select closest hit bone

#### Color-Based Picking (GPU)
1. Render each bone with unique color ID
2. Read back pixel under cursor
3. Map color to bone index
4. Advantage: exact shape matching
5. Disadvantage: GPU readback latency

#### Bounding Volume Approach
- AABB per bone for fast rejection
- Then precise intersection test
- Works well with existing physics/collision systems

### 6.5 Bone Orientation/Roll Visualization
- Y axis always along bone (root to tip)
- Roll rotation around Y axis shown by:
  - Octahedral: square cross-section rotation
  - Axis display: local XYZ axes drawn at bone
  - Tripod: small RGB lines at joint
- Blender: "Axes" display option shows local axes per bone

### 6.6 IK Chain and Constraint Visualization
- IK chains: draw differently colored lines along chain
- Effector targets: gizmo shapes (sphere, cross, diamond) at target position
- Pole targets: line from chain midpoint to pole target
- Constraint limits: arc/cone gizmos showing rotation limits
- Blender: IK limit overlay with potential visual artifacts

### 6.7 Hierarchy Lines
- Dashed or dotted lines connecting parent to child joints
- Blender: relationship lines (not visible for all display types)
- Typically drawn as simple GL_LINES or VK_PRIMITIVE_TOPOLOGY_LINE_LIST
- Color often matches bone theme or uses neutral gray


## 7. Vulkan-Specific Implementation Guide

### 7.1 Pipeline Setup for Bone Overlay

#### Option A: Separate Render Pass
```
Pass 1 (Scene):
  - Color attachment: LOAD_CLEAR, STORE_STORE
  - Depth attachment: LOAD_CLEAR, STORE_STORE
  - Normal depth test + write

Pass 2 (Overlay):
  - Color attachment: LOAD_LOAD, STORE_STORE (preserve scene)
  - Depth attachment: LOAD_LOAD (read scene depth)
  - Depth test disabled OR depth test without write
```

#### Option B: Subpass within Same Render Pass
```
Subpass 0 (Scene): normal rendering
Subpass 1 (Overlay): bone gizmos
  - Subpass dependency ensures depth is available
  - Uses input attachment for scene depth (if needed)
```

#### Option C: Same Pass, Different Pipeline State
```
Single render pass:
  1. Bind scene pipeline -> draw scene
  2. Bind overlay pipeline (depth test off) -> draw bones
  - Simplest approach
  - Works well if bones are always drawn last
```

### 7.2 Line Rendering in Vulkan

#### Native Lines
- `VK_PRIMITIVE_TOPOLOGY_LINE_LIST` or `LINE_STRIP`
- Default line width: 1.0 (only guaranteed width)
- Wide lines: requires `wideLines` GPU feature
- Bresenham lines: `VK_EXT_line_rasterization` extension
- Line stipple: `lineStipplePattern` and `lineStippleFactor`

#### Triangle-Based Lines (More Portable)
- Expand lines to quads in vertex shader
- Screen-space width (constant pixel width)
- No GPU feature dependency
- Im3d technique: instanced triangle strip per line segment
- More complex but guaranteed to work everywhere

### 7.3 Instanced Bone Rendering
```
Per-instance data (push constant or SSBO):
  - mat4 bone_transform (bone world matrix)
  - vec4 bone_color (RGBA)
  - float bone_length (for scaling)

Vertex shader:
  - Fetch bone shape vertex from VBO
  - Scale by bone_length along Y
  - Transform by bone_transform
  - Apply view-projection

Fragment shader:
  - Output bone_color
  - Optional: edge detection for outline
  - Optional: simple N-dot-L shading for solid bones
```

### 7.4 Recommended Implementation Order
1. Start with Stick bones (lines between joints) - simplest
2. Add joint dots (small quads or instanced spheres)
3. Add octahedral solid bones (instanced mesh)
4. Add wireframe overlay on solid bones
5. Add color coding and selection highlighting
6. Add picking via ray casting
7. Add "In Front" toggle (depth test on/off)
8. Add custom shapes (user-defined mesh per bone)


## References

- [Blender Viewport Display Manual](https://docs.blender.org/manual/en/latest/animation/armatures/properties/display.html)
- [Blender Overlay Engine Source](https://projects.blender.org/blender/blender) - `source/blender/draw/engines/overlay/`
- [Blender Armature Drawing Refactor PR #106232](https://projects.blender.org/blender/blender/pulls/106232)
- [Blender Bone Color Proposals](https://projects.blender.org/blender/blender/issues/112635)
- [Maya Joints and Bones](https://help.autodesk.com/view/MAYAUL/2025/ENU/?guid=GUID-1B59334F-2605-44C3-B584-A55B239A2CBE)
- [Maya X-Ray Joints](https://download.autodesk.com/global/docs/maya2014/en_us/files/Shading__XRay_Joints.htm)
- [MotionBuilder Character Controls](https://knowledge.autodesk.com/support/motionbuilder/learn-explore/caas/CloudHelp/cloudhelp/2022/ENU/MotionBuilder/files/GUID-F3374419-7FF9-40A6-B8D4-6D012797A878-htm.html)
- [3ds Max Bones System](https://knowledge.autodesk.com/support/3ds-max/learn-explore/caas/CloudHelp/cloudhelp/2015/ENU/3DSMax/files/GUID-E6164716-CFA9-4DE9-9976-F8A58850461F-htm.html)
- [UE DebugDrawSkeleton](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/FSkinnedSceneProxy/DebugDrawSkeleton)
- [UE Skeleton Editor](https://dev.epicgames.com/documentation/en-us/unreal-engine/skeleton-editor-in-unreal-engine)
- [Unity BoneRenderer](https://docs.unity3d.com/Packages/com.unity.animation.rigging@1.1/api/UnityEngine.Animations.Rigging.BoneRenderer.html)
- [Unity Animation Rigging](https://docs.unity3d.com/Packages/com.unity.animation.rigging@1.0/manual/index.html)
- [Godot Skeleton Editor Gizmo PR #45699](https://github.com/godotengine/godot/pull/45699)
- [Godot SkeletonModifier3D Design](https://godotengine.org/article/design-of-the-skeleton-modifier-3d/)
- [Bevy Gizmos API](https://docs.rs/bevy_gizmos)
- [Im3d - Immediate Mode 3D Gizmos](https://github.com/john-chapman/im3d)
- [Vulkan Depth Buffering Tutorial](https://vulkan-tutorial.com/Depth_buffering)
- [Vulkan Line Rasterization](https://docs.vulkan.org/samples/latest/samples/extensions/dynamic_line_rasterization/README.html)
- [Godot Gizmo Overlay Proposal #2138](https://github.com/godotengine/godot-proposals/issues/2138)
