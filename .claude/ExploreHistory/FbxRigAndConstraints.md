# FBX Rig and Constraints Investigation

## Summary

Investigated FBX rig/skeleton structure, animation data, and constraints system for an **animation editing + rendering
engine** (not a game engine). Constraints must be stored as raw data and evaluated at runtime, enabling interactive
editing of IK, Aim, Parent constraints etc. Baking is only performed as a final export step. Migrating from
fbxcel-dom + russimp to the ufbx crate would improve FBX support significantly.

---

## Project Context

This project is an **animation editing + rendering engine**, not a game engine. This means:

- Constraints are stored as **raw editable data**, not baked keyframes
- Users must be able to add, remove, and tweak constraint parameters interactively
- Baking constraints to keyframes is an **export-time operation**, not an import-time one
- The constraint evaluation pipeline must run every frame during editing

This differs from game engines (Unreal, Unity) where constraints are typically baked at export.

Reference: Blender, Maya, MotionBuilder handle constraints as live-evaluated raw data during editing.

---

## Current Project FBX Loader Status

The FBX loader (`src/loader/fbx/`) uses a dual strategy:

- **russimp** (Assimp bindings): Primary loader for mesh, bones, animations
- **fbxcel-dom**: Supplementary animation curve loading (used when more keyframes detected)
- Skeletal animation and node hierarchy animation are working
- **Not supported**: Morph targets, constraints, PBR materials

### Key Files

| File | Role |
|------|------|
| `src/loader/fbx/fbx.rs` (2933 lines) | Core FBX loading with russimp and fbxcel-dom |
| `src/loader/fbx/loader.rs` (546 lines) | Conversion to graphics resources |
| `src/debugview/fbx_debug.rs` | Debug flags for FBX issues |

### Data Structures

- `FbxModel`: Contains `Vec<FbxData>`, `Vec<FbxAnimation>`, `HashMap<String, BoneNode>`, `unit_scale`
- `FbxData`: Mesh data with positions, normals, indices, clusters, mesh_parts
- `BoneNode`: Name, parent, local_transform, default TRS values
- `FbxAnimation`: Name, duration, `HashMap<String, BoneAnimation>`
- `ClusterInfo`: bone_name, transform, transform_link, inverse_bind_pose, vertex_indices, vertex_weights

---

## FBX Rig Structure

### Scene Graph and Skeleton

FBX scenes are tree structures of `FbxNode`. Skeletons are a semantic specialization of this tree:

```
FbxScene
  └── FbxNode tree (scene graph)
        └── FbxSkeleton nodes (eRoot / eLimb / eEffector)
              └── FbxSkin deformer (attached to mesh)
                    └── FbxCluster (bone-to-vertex binding)
                          ├── vertex_indices + weights
                          └── geometry_to_bone (inverse bind matrix)
```

### FbxSkeleton Types

- **eRoot**: Root of skeleton hierarchy
- **eLimb**: Intermediate limb node (e.g., thigh, shin)
- **eEffector**: End of chain (e.g., ankle)

### Deformer Types

| Deformer | Description |
|----------|-------------|
| Skin Deformer | Classic vertex-to-bone weight binding |
| Blend Deformer | Blend shape / morph target vertex offsets |
| Cache Deformer | Replace mesh data from disk cache |

### Skinning Methods

- **Rigid**: 1 bone per vertex
- **Linear**: Classic Linear Blend Skinning (LBS)
- **Dual Quaternion**: DQ skinning
- **Blended DQ/Linear**: Per-vertex blend of LBS and DQ

### Cluster Data (Skin Cluster)

FBX stores skinning as bone→vertices (opposite of GPU format vertex→bones):

- `geometry_to_bone`: Mesh space to bone local space transform (inverse bind matrix)
- `vertices[]`: Affected vertex indices
- `weights[]`: Weight per vertex

Conversion to GPU format requires:

1. Iterate all clusters
2. Build per-vertex bone/weight pairs
3. Sort by weight, take top 4
4. Normalize weights to sum to 1.0

---

## FBX Animation Data Structure

### Hierarchy (top → bottom)

```
AnimStack (= 1 animation clip / take)
  └── AnimLayer (blend layer)
        └── AnimCurveNode (property-curve connection point)
              └── AnimCurve (FCurve: time → value)
                    └── AnimCurveKey (keyframe + interpolation type)
```

### Class Roles

| Class | Role |
|-------|------|
| AnimStack | Top container. 1 AnimStack = 1 animation clip |
| AnimLayer | Container for AnimCurveNodes. At least 1 (base layer) required |
| AnimCurveNode | Connection between AnimCurve and FBX property. For 3-component properties (e.g., LclTranslation), holds X/Y/Z curves |
| AnimCurve | Actual animation curve (FCurve) defining value over time |
| AnimCurveKey | Individual keyframe with time, value, and interpolation type (constant, linear, cubic) |

### Animation Evaluation Options (ufbx)

- `bake_anim()`: Convert curves to simple linearly-interpolated tracks (auto-handles cubic resampling, Euler→Quaternion)
- `evaluate_scene()`: Returns new scene with all animations applied at given time
- `evaluate_transform()`: Evaluate specific node TRS at time
- `evaluate_curve()`: Evaluate single curve at time

---

## FBX Constraints System

### Supported Constraint Types (FBX SDK)

| Type | Description |
|------|-------------|
| Position | Constrain Translation |
| Rotation | Constrain Rotation |
| Scale | Constrain Scale |
| Parent | Follow parent (Translation + Rotation) |
| Aim | Rotate toward target |
| SingleChainIK | 2-bone IK chain |

### SingleChainIK Properties

- Pole Vector (default: 0, 1, 0)
- Twist value
- First Joint, End Joint, Effector references

---

## Runtime Constraint Architecture (for Animation Editor)

### Design Principles

Since this is an animation editor, constraints must be:

1. **Stored as raw data**: Constraint type, parameters, source/target references
2. **Evaluated every frame**: Results applied to bone poses before skinning
3. **Editable in UI**: Users can add/remove/modify constraints interactively
4. **Order-dependent**: Evaluation order matters (e.g., IK after FK, Aim after Position)
5. **Bakeable on export**: Convert evaluated results to keyframes when exporting
6. **Strict ECS architecture**: Follow the project's existing ECS patterns without exception
   - **Component**: Data-only structs attached to entities. Components go in `ecs/component/`
   - **System**: All logic is implemented as system functions in `ecs/systems/`. No `impl` blocks
     with evaluation logic on components
   - **Resource**: Global dynamic state goes in `ecs/resource/`
   - **Query**: Find entities via marker components and query functions
   - **EntityBuilder**: Use the existing EntityBuilder pattern (`src/ecs/world.rs:484`) for entity
     construction. This project does NOT use a `ecs/bundle/` directory
   - **Bones are NOT entities**: Bones are data within `Skeleton.bones: Vec<Bone>`, identified by
     `BoneId` (u32 index). Constraints referencing bones use `BoneId`, not `Entity`
   - **Constraint ownership**: Constraints are attached to the **model entity** as a component
     (like `ClipSchedule`, `AnimationMeta`), since bones themselves are not entities
   - Reference: [Bevy Engine ECS](https://github.com/bevyengine/bevy) for pattern guidance

### Proposed Animation Phase Pipeline

```
Animation Phase (per frame):
  1. Sample AnimationClip → BoneLocalPose (FK pose)
  2. Compute global transforms (pre-constraint)
  3. Evaluate constraints in priority order:
     a. Parent constraints
     b. Position / Rotation / Scale constraints
     c. Aim constraints
     d. IK constraints (requires global positions)
  4. Recompute global transforms (post-constraint)
  5. Apply skinning to vertices
```

IK and Aim constraints require world-space bone positions as input, so global transforms must be computed
before these constraints and recomputed after.

### ECS Integration

#### Constraint Data Structures (new, in `animation/`)

Individual constraint definitions are plain data structs stored in the animation module
(alongside Skeleton, AnimationClip, etc.), since they operate on BoneId, not on entities:

```rust
pub struct IkConstraintData {
    pub enabled: bool,
    pub chain_length: u32,
    pub target_bone: BoneId,
    pub effector_bone: BoneId,
    pub pole_vector: Vector3<f32>,
    pub pole_target: Option<BoneId>,
    pub twist: f32,
    pub weight: f32,
}

pub struct AimConstraintData {
    pub enabled: bool,
    pub source_bone: BoneId,
    pub target_bone: BoneId,
    pub aim_axis: Vector3<f32>,
    pub up_axis: Vector3<f32>,
    pub up_target: Option<BoneId>,
    pub weight: f32,
}

pub struct ParentConstraintData {
    pub enabled: bool,
    pub constrained_bone: BoneId,
    pub sources: Vec<(BoneId, f32)>,
    pub affect_translation: [bool; 3],
    pub affect_rotation: [bool; 3],
    pub weight: f32,
}

pub struct PositionConstraintData {
    pub enabled: bool,
    pub constrained_bone: BoneId,
    pub target_bone: BoneId,
    pub offset: Vector3<f32>,
    pub affect_axes: [bool; 3],
    pub weight: f32,
}

pub struct RotationConstraintData {
    pub enabled: bool,
    pub constrained_bone: BoneId,
    pub target_bone: BoneId,
    pub offset: Quaternion<f32>,
    pub affect_axes: [bool; 3],
    pub weight: f32,
}

pub struct ScaleConstraintData {
    pub enabled: bool,
    pub constrained_bone: BoneId,
    pub target_bone: BoneId,
    pub offset: Vector3<f32>,
    pub affect_axes: [bool; 3],
    pub weight: f32,
}
```

#### ConstraintSet Component (new, in `ecs/component/`)

A single component attached to the **model entity** that owns all constraints for that model.
This follows the same pattern as `ClipSchedule` (per-entity, contains Vec of data):

```rust
pub enum ConstraintType {
    Ik(IkConstraintData),
    Aim(AimConstraintData),
    Parent(ParentConstraintData),
    Position(PositionConstraintData),
    Rotation(RotationConstraintData),
    Scale(ScaleConstraintData),
}

pub struct ConstraintEntry {
    pub constraint: ConstraintType,
    pub priority: u32,
}

pub struct ConstraintSet {
    pub constraints: Vec<ConstraintEntry>,
}
```

Evaluated in priority order. If constraint A's output feeds constraint B's input, A must have
higher priority. Systems access this via `world.get_component::<ConstraintSet>(model_entity)`.

### IK Solver Algorithm

For 2-bone IK (SingleChainIK), the standard approach:

1. Get world positions of root, mid, end joints
2. Get target position
3. Compute distances: root→mid, mid→end
4. Use law of cosines to find mid-joint bend angle
5. Apply pole vector to determine bend plane
6. Convert back to local rotations for each joint

Reference implementations:
- [Bevy Engine animation crate](https://github.com/bevyengine/bevy/tree/main/crates/bevy_animation)
- [MotionBuilder Constraint Documentation](https://help.autodesk.com/view/MOBPRO/2025/ENU/?guid=GUID-A50E57F8-CF3A-4AA4-B660-64C045E90F3E)
- [Two Bone IK algorithm (GDC)](https://www.gdcvault.com/play/1024949/Rig-Constraints-for-Organic)

### Bake-to-Keyframes (Export Feature)

When exporting, convert constraint-evaluated poses to keyframes:

1. For each frame in the animation range:
   a. Sample FK animation
   b. Evaluate all constraints
   c. Record final bone TRS as keyframes
2. Output as standard AnimationClip (constraint-free)

This produces animation data compatible with any game engine import.

---

## How Other Engines Handle FBX Rigs

### Bevy Engine

- No native FBX support yet
- `bevy_ufbx` crate (latest: 0.17.0, 2026-01-04): Uses ufbx Rust bindings
- Standard workflow: FBX → glTF conversion via Blender
- Native support discussed in [GitHub Issue #15705](https://github.com/bevyengine/bevy/issues/15705)

### Unreal Engine

- FBX is the primary skeletal mesh exchange format (FBX 2020.2)
- Auto-generates Skeleton Asset from bone hierarchy on import
- Shares skeletons between meshes with matching bone names/hierarchy
- Constraints NOT preserved from FBX; rebuilt in Animation Blueprint
- Root bone = skeletal mesh pivot point

### Blender (Animation Editor Reference)

- Constraints are raw data, evaluated every frame during editing
- Armature constraints include: IK, Copy Location/Rotation/Scale, Track To, Damped Track, etc.
- Constraint stack per bone, evaluated top to bottom
- "Bake Action" operator converts constraints to keyframes for export
- Bone direction correction needed for FBX: FBX bones use -X, Blender uses Y
- Axis: Blender Y Forward / Z Up vs standard -Z Forward / Y Up

### Maya / MotionBuilder (Animation Editor Reference)

- Constraints stored as dependency graph nodes
- Evaluated in dependency order each frame
- Full Rig = skeleton + constraints + controllers
- FBX export can optionally bake or preserve constraint data

### Pixar Presto (Animation Editor Reference)

Pixar's in-house animation software, used since "Brave" (2012). Not commercially available.
Academy Sci-Tech Award winner (2018). Most relevant reference for this project as a professional
animation editing tool.

**Architecture**:
- Node-based rig architecture: rigs are "operator graphs" evaluated by a parallel execution engine
- DAG-based evaluation (Directed Acyclic Graph): all computations are nodes with data-flow edges
- Value-driven re-evaluation: cache invalidated on input change, re-computed on demand (not event-driven)
- Static dependency graph: topology cannot change during evaluation, ensuring predictability
- Scales to millions of evaluation nodes with heavy multithreading (node/branch/model/frame level)
- Core in C++, scripting in Python, rendering via Hydra + RenderMan

**Rig System**:
- Rig compiler: high-level rig objects compiled to optimized data structures for efficient deformation
- Direct manipulation paradigm: pose characters by manipulating the mesh itself (no visible NURBS/bones)
- Invertible Rig (SIGGRAPH Asia 2024): each rig component runs bidirectionally (FK↔IK seamlessly)
- Neural IK (SIGGRAPH Asia 2023, with Disney Research): ML-based IK solver using sparse controls,
  preserves previous edits, exploits full-body correlations to fill uncontrolled joints
- Sketch-to-Pose (SIGGRAPH 2015): multi-joint posing via single stroke for chains (neck, tail, etc.)

**Constraint Evaluation**:
- Constraints evaluated as part of the DAG execution network
- Supports: constraint target computation, transform hierarchy, attribute inheritance chains, dataflow
- IK with "reach constraints": define multiple reach goals, enforced during manipulation
- Redundancy and singularity handling built into IK solver
- Evaluation order determined by graph dependencies (not manual priority)

**Deformation & Skinning**:
- Linear Blend Skinning (LBS) and Dual Quaternion Skinning (DQS)
- Sculpting brush: layered sculpts on top of posed/unposed models, animatable on timeline
- Blend shapes for facial animation, with ML approximation of rig-driven deformation
- Mesh-free rigs (Elio, 2025): characters built entirely from signed distance functions

**Scene Description (USD origin)**:
- USD originated from Presto's internal scene composition engine
- Layering, referencing, variants, inheritance, overrides at any granularity
- Composition strength order: LIVRPS (Locals, Inherits, VariantSets, References, Payloads, Specializes)
- Multi-artist collaboration: different artists work on different layers of the same scene simultaneously
- OpenExec = open-source version of Presto's execution system, built on USD

**Key Takeaways for This Project**:
- Constraints as part of evaluation graph (not separate phase) enables natural dependency resolution
- Invertible rig concept: FK↔IK bidirectional conversion is valuable for animation editors
- Rig compilation step: optimize rig for evaluation performance (separate authoring vs runtime representation)
- Value-driven + cache invalidation pattern aligns well with ECS resource change detection

**References**:
- [Sketch to Pose (SIGGRAPH 2015)](https://dl.acm.org/doi/10.1145/2775280.2792583)
- [Pose and Skeleton-aware Neural IK (SIGGRAPH Asia 2023)](https://dl.acm.org/doi/10.1145/3610548.3618217)
- [Invertible Rigs + ML Posing (SIGGRAPH Asia 2024)](https://dl.acm.org/doi/10.1145/3681757.3697056)
- [OpenExec Documentation](https://openusd.org/dev/intro_to_openexec.html)
- [Multithreading in Presto (SIGGRAPH Asia 2019)](https://sa2019.siggraph.org/attend/courses/session/18/details/28)
- [Presto Sculpting Brush (AWN article)](https://www.awn.com/animationworld/piper-and-development-pixar-s-presto-sculpting-brush)

---

## Constraint UI Patterns in Industry Tools

### Common Pattern (Maya, Blender, Unity)

All major tools share the same fundamental workflow:

1. **Constrained object = current selection** — user selects a bone, then adds a constraint to it
2. **Target starts EMPTY** — no default bone auto-assigned; constraint is non-functional until user sets target
3. **Constraint has no effect until configured** — shown in red/disabled state until target is specified
4. **Bone selection via dropdown/eyedropper** — user explicitly picks target bone from dropdown or viewport

### Blender (Copy Location Constraint)

- Workflow: Select bone in Pose Mode → Bone Constraints panel → Add Bone Constraint → Copy Location
- **Target field starts empty** (constraint shows red/non-functional)
- Target selection: Armature dropdown → Bone dropdown (appears after armature selected)
- Head/Tail slider for precise target point on bone
- Influence (weight) slider: 0.0 - 1.0
- Axis filtering: X, Y, Z checkboxes with Invert options
- Space: World / Local / Pose / Custom
- Reference: [Copy Location Constraint](https://docs.blender.org/manual/en/latest/animation/constraints/transform/copy_location.html)

### Maya (Point Constraint)

- Workflow: Select driver → Shift+select driven → Constrain → Point
- **Selection order determines roles** (no dropdown during creation)
- Options panel: Maintain Offset toggle, All Axes / specific axis toggles
- Weight per target object (default 1.0)
- Post-creation editing via Channel Box / Attribute Editor
- Reference: [Point Constraint](https://help.autodesk.com/view/MAYAUL/2024/ENU/?guid=GUID-DAE54462-3F5B-4F5E-B038-9482491B5429)

### Unity (Animation Rigging - Position Constraint)

- Component-based: attach Position Constraint component to GameObject
- Source Objects: empty array → add via drag-and-drop or object picker
- Constrained Object: the GameObject the component is on
- Weight per source object
- Reference: [Constraint Components](https://docs.unity3d.com/Packages/com.unity.animation.rigging@1.1/manual/ConstraintComponents.html)

### Key Takeaway for This Project

Our current implementation auto-assigns default bones and immediately enables the constraint. This causes
destructive behavior (e.g., root bone constrained to leaf bone). Industry standard is:

- **Add with `enabled: false`** — constraint created in inactive state
- **Both bone fields default to 0** — harmless since constraint is disabled
- **User selects bones via dropdown, then enables** — explicit user action required
- **No `pick_default_bones` logic needed** — remove auto bone selection entirely

---

## Rust FBX Parsing Crates Comparison

### Currently Used

| Crate | Version | Status |
|-------|---------|--------|
| fbxcel | 0.9.0 | Low-level FBX parser |
| fbxcel-dom | 0.0.10 | DOM API. **No future updates planned** |
| russimp | 3.2 | Assimp bindings (primary loader) |

### Alternative: ufbx

| Aspect | ufbx |
|--------|------|
| Crate | `ufbx` on crates.io |
| GitHub | [ufbx/ufbx-rust](https://github.com/ufbx/ufbx-rust) |
| Base | C library, single source file |
| Skeleton/Skin | Full support: SkinDeformer, SkinCluster, per-vertex weights |
| Animation | Full support: AnimStack, AnimLayer, bake_anim(), curve eval |
| Blend Shapes | Supported |
| Constraints | Can read FBX constraint objects as raw data |
| Maintenance | Actively developed |
| Bevy integration | bevy_ufbx crate |

**ufbx provides significantly richer high-level APIs compared to fbxcel-dom, and can preserve raw constraint data.**

---

## Unified Implementation Plan (Rig Visualization + Constraints)

This plan integrates rig visualization (from RigVisualizationUI.md) and constraint system implementation
into a single roadmap. All phases follow strict ECS architecture.

### Phase 1: Wireframe Bone Visualization (Stick)

Foundation for all subsequent rig work.

- Use existing Gizmo pipeline (`LINE_LIST`, no depth test, `src/app/init/instance.rs:215-229`)
- Draw lines from parent joint to child joint using `ColorVertex` + `LineMesh`
- Draw small cross/dot at each joint position
- ECS component on model entity: `BoneGizmoState` (display style, selection, per-bone colors, visibility)
- ECS systems: `build_bone_gizmo_meshes()`, `update_bone_gizmo_transforms()`
- ECS resource: `BoneDisplayConfig` (global display settings)
- Entity construction: extend EntityBuilder with `.with_bone_gizmo_state()`

### Phase 2: Octahedral Solid Bones

Blender-style dual-pyramid bone display.

- New Vulkan pipeline: `BoneSolid` (TRIANGLE_LIST, FILL, depth test OFF, depth write OFF)
- Generate octahedral mesh (6 verts, 8 tris) scaled by bone length per bone
- Wireframe outline via `BoneWire` pipeline (LINE_LIST, LINE, depth test OFF)
- Bone transform: `model_matrix = global_transform * scale_by_length * roll_rotation`
- Octahedral geometry definition: see RigVisualizationUI.md for vertex/triangle spec

### Phase 3: Bone Selection and Interaction

- CPU ray casting via existing `screen_to_world_ray()` (`src/math/coordinate_system.rs:99`)
- Add `ray_to_triangle_intersection()` for octahedral bone face hit test
- Color highlight: normal (theme color), selected (brighter), active (accent)
- G-Buffer Object ID supplementary: render bones to G-Buffer with unique IDs
  to reuse existing selection outline rendering (`compositeFragment.frag:48-67`)
- ECS systems: `select_bone_by_ray()`, `highlight_selected_bones()`

### Phase 4: Constraint Data Model

- Define constraint data structs in `animation/` (alongside Skeleton, AnimationClip):
  - `IkConstraintData`, `AimConstraintData`, `ParentConstraintData`
  - `PositionConstraintData`, `RotationConstraintData`, `ScaleConstraintData`
  - All data-only structs, no evaluation logic
- Define `ConstraintSet` component in `ecs/component/` (attached to model entity,
  same pattern as `ClipSchedule`)
- Add `Constrained` marker component in `ecs/component/marker.rs`
- Extend EntityBuilder with `.with_constraint_set()`
- Import constraint raw data from FBX (via ufbx or manual fbxcel parsing)

### Phase 5: Runtime Constraint Evaluation
- Implement constraint solver systems in `ecs/systems/`
- Insert into animation phase pipeline:
  ```
  1. Sample AnimationClip → BoneLocalPose (FK)
  2. Compute global transforms (pre-constraint)
  3. Evaluate constraints in priority order:
     a. Parent constraints
     b. Position / Rotation / Scale constraints
     c. Aim constraints
     d. IK constraints (requires global positions)
  4. Recompute global transforms (post-constraint)
  5. Apply skinning to vertices
  ```
- Start with 2-bone IK (most impactful), then Position/Rotation, then Aim/Parent

### Phase 6: Constraint Visualization

- IK chain: differently colored lines along IK chain
- Effector gizmos: sphere/diamond shapes at IK target positions
- Pole vector: line from mid-joint to pole target
- Rotation limit cones: arc gizmo showing allowed rotation range
- Reuse existing Gizmo pipeline for line-based visualizations
- New gizmo shapes (sphere, cone) share `BoneSolid` pipeline

### Phase 7: Constraint UI Editing (Implemented)

- Constraint inspector panel in ImGui
- Add/remove constraints with interactive parameter editing
- Inspector auto-resolves Animator entity (not selected_entity)
- Multi-mesh models: shared ConstraintSet applied to all Animator entities' poses
- Visual feedback: gizmo updates in real-time as parameters change

### Phase 7.1: Constraint UI Industry Standard Compliance

- Create constraints with `enabled: false` (matching Blender/Maya/Unity pattern)
- Bone selection via dropdown only — no auto bone selection on creation
- User selects bones via dropdown, then manually enables the constraint
- Remove `pick_default_bones` / `find_mid_hierarchy_pair` auto-selection logic

### Phase 8: Bake-to-Keyframes Export

- Evaluate full animation range with constraints applied
- Record per-frame bone TRS as keyframes
- Output as constraint-free AnimationClip for game engine compatibility

### Phase 9: Advanced Visualization Features

- "In Front" toggle: switch bone pipeline depth test ON/OFF
- Two-pass rendering: solid when visible, transparent when occluded
- Custom bone shapes: user-defined mesh per bone
- Distance-based scaling (like existing LightGizmoData)

### Parallel: FBX Loader Migration

- Migrate from fbxcel-dom + russimp to ufbx crate
- ufbx provides raw constraint data access, better animation curves, blend shape support
- Single dependency replaces three
- Can begin at any point; benefits Phase 4 (constraint import) most directly
