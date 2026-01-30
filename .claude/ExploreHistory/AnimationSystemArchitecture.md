# Animation System Architecture - Cross-Engine & DCC Tool Comparison

## Summary

This document compares animation system architectures across major game engines (Unity, Unreal, Godot, Bevy), DCC tools (Blender, Maya, 3ds Max, Houdini, Cinema 4D), Pixar's proprietary Presto system, and includes the proposed architecture for this project. Key findings:

- **Game engines** split into State Machine (Unity), Dual-Graph (Unreal), Data/Control Separation (Godot), and ECS+DAG (Bevy) patterns
- **DCC tools** universally use Non-Linear Animation (NLA) systems for clip composition, with varying degrees of proceduralism
- **Pixar Presto** uses a unified composition engine (which became USD), multithreaded rig solving, and Hydra-based viewport rendering
- **Export pipeline** is almost always destructive: rich DCC data is baked to flat keyframes for runtime efficiency
- **This project** adopts Blender's "Data/Control Separation" + Bevy's "ECS Pattern": ClipLibrary (asset data) + Animator (per-entity component) + system evaluation functions, without state machines or blend graphs

---

## Part 1: Game Engines

### 1.1 Unity -- State Machine + Playable API

#### Core Architecture

```
AnimatorController (Asset)
  +-- Layer(s)
        +-- StateMachine
              +-- State "Idle" -> AnimationClip
              +-- State "Walk" -> AnimationClip
              +-- Transition (condition: speed > 0.1)

Timeline (Separate System, built on Playable API)
  +-- AnimationTrack -> bound to Animator
        +-- TimelineClip -> AnimationPlayableAsset -> AnimationClip
```

#### Animation Clip Storage

- `AnimationClip` is a project Asset containing multiple animation curves (`EditorCurveBinding`)
- Two types: Generic (specific to a GameObject hierarchy) and Humanoid (retargetable)
- Each curve stores time-series data for Transform or arbitrary properties

#### Animator Controller

- **Finite State Machine** at its core
- Each State references an AnimationClip
- Transitions fire based on parameter conditions (float, int, bool, trigger)
- `AnimatorOverrideController` allows swapping clips without modifying the state machine
- **BlendTree**: Blends multiple clips based on parameters (speed, direction)
- **Layers**: Body-part isolation via Avatar Masks (e.g., upper body override, lower body base)

#### Entity Binding

- `Animator` component attached to a GameObject
- On first evaluation, **Rebinding** occurs: `EditorCurveBinding` paths are resolved to target Components
- Rebinding triggers on: Controller swap, Prefab instantiation, explicit `Animator.Rebind()`

#### Blending & Transitions

- **Crossfade**: Linear or curve-based blending over specified duration
- **BlendTree**: Parameter-driven multi-clip blending
- **Layer Blending**: Additive or Override per layer with Avatar Masks
- **Playable API**: Dynamic blend graph construction via `AnimationMixerPlayable`

#### Timeline vs AnimationClip

- Timeline is a **cinematic sequencer** built on Playable API
- `AnimationTrack` binds to Animator via `[TrackBindingType(typeof(Animator))]`
- `TimelineClip` wraps `AnimationPlayableAsset` which references `AnimationClip`
- AnimatorController handles **runtime state transitions**, Timeline handles **time-axis arrangement**

---

### 1.2 Unreal Engine -- Dual-Graph + AnimMontage

#### Core Architecture

```
Animation Blueprint
  +-- EventGraph  -> Variable updates, game logic (Update phase)
  +-- AnimGraph   -> State machines, BlendSpaces, pose composition (Evaluate phase, parallelizable)

AnimMontage (Gameplay-driven)
  +-- Section/Slot -> wraps AnimSequence

Sequencer (Cinematic)
  +-- Time-axis control for cutscenes
```

#### Animation Data

- `UAnimSequence`: Single animation asset bound to a Skeleton
- Stores per-bone keyframe data (position, rotation, scale)
- Skeleton binding enables reuse across multiple SkeletalMeshes sharing the same Skeleton
- `UAnimMontage`: Structured wrapper with Sections and Slots for gameplay-driven playback

#### Animation Blueprint

- **EventGraph**: Standard Blueprint for variable updates and game logic
- **AnimGraph**: Animation-specific node graph with state machines, BlendSpaces, pose composition
- `UAnimInstance`: Runtime instance, one per SkeletalMeshComponent

Execution flow:
1. **Initialize**: First use or mesh change
2. **Update (EventGraph)**: Variable updates
3. **Evaluate (AnimGraph)**: Pose calculation (parallelizable, highest cost)

#### Entity Binding

- `SkeletalMeshComponent` has an Animation Mode:
  - `Use Animation Asset`: Direct AnimSequence playback
  - `Use Animation Blueprint`: Full controller
- AnimSequence is bound to Skeleton, not SkeletalMesh

#### Blending & Transitions

- **Transition Rules**: Bool-output conditions for state transitions
- **Blend Types**: Standard, Inertialization (inertia-based), Custom
- **BlendSpace**: 1D/2D parameter-space interpolation
- **Layer Blend**: Additive, per-bone override
- **Conduit**: Many-to-many transition logic
- **Linked Anim Layers**: Dynamically load/unload Animation Blueprint modules

#### AnimMontage

- Unique concept combining clips with gameplay control
- **Sections**: Named segments for branching (e.g., combo chains)
- **Slots**: Named channels for body-part isolation
- Controllable from code/Blueprint: `PlayMontage()`, `JumpToSection()`, etc.
- Closest equivalent to Unity's Timeline for gameplay animation

---

### 1.3 Godot -- Data/Control Separation

#### Core Architecture

```
AnimationPlayer (Data Storage)
  +-- Animation "idle"   <- Can animate almost any node property
  +-- Animation "walk"
  +-- Animation "run"

AnimationTree (Playback Control)
  +-- root: StateMachine or BlendTree
  +-- anim_player -> references AnimationPlayer
```

#### Animation Data

- `AnimationPlayer` node directly holds animation data
- `Animation` resource with tracks: Transform, Bezier, Method Call, Audio, Sub-Animation
- **NodePath-based** property access: can animate virtually any property on any node
- This universality is unique among engines

#### Control Layer

- **AnimationTree**: Pure playback control, references AnimationPlayer for data
- Root node types:
  - `AnimationNodeStateMachine`: State machine with A* pathfinding for `travel()`
  - `AnimationNodeBlendTree`: Blend tree with various blend nodes
  - `AnimationNodeBlendSpace2D/1D`: Parameter-space blending

#### Blending & Transitions

- **Crossfade**: `xfade_time` and `xfade_curve`
- **Transition modes**: Immediate, Sync, At End
- **Blend nodes**: Blend2, Blend3, OneShot, Add2, TimeScale, Seek, Transition
- **Advance Expression (Godot 4)**: Arbitrary expressions for transition conditions

#### Timeline

- AnimationPlayer itself **embeds timeline functionality**
- No separate Timeline system exists
- Cutscene control is done directly via AnimationPlayer

---

### 1.4 Bevy (Rust) -- ECS + DAG Graph

#### Core Architecture

```rust
// AnimationClip = Asset
pub struct AnimationClip {
    curves: AnimationCurves,
    events: AnimationEvents,
    duration: f32,
}

// AnimationGraph = Asset (petgraph DAG)
pub struct AnimationGraph {
    pub graph: AnimationDiGraph,    // Clip / Blend / AdditiveBlend nodes
    pub root: NodeIndex,
    pub mask_groups: HashMap<AnimationTargetId, AnimationMask>,
}

// AnimationPlayer = Component
// AnimationTransitions = Component
```

#### Crate Structure (bevy_animation)

```
bevy_animation/src/
+-- lib.rs              - AnimationClip, AnimationPlayer, AnimationTarget
+-- graph.rs            - AnimationGraph (DAG), AnimationGraphNode
+-- transition.rs       - AnimationTransitions (fade transitions)
+-- animation_curves.rs - AnimationCurve trait, AnimatableCurve
+-- animation_event.rs  - Animation events
+-- animatable.rs       - Animatable trait, AnimatableProperty
+-- gltf_curves.rs      - glTF-specific curve processing
+-- morph.rs            - Morph target animation
+-- util.rs             - Utilities
+-- macros/             - Procedural macros
```

#### Animation Data

- `AnimationClip` is an **Asset** managed via `Handle<AnimationClip>`
- Curves use `VariableCurve` (wrapping `Box<dyn AnimationCurve>`)
- `AnimatableCurve<P, C>` binds a property type to a curve type
- glTF import handled by `gltf_curves.rs`

#### Animation Graph

- **DAG (Directed Acyclic Graph)** based on `petgraph`, not a state machine
- Three node types: **Clip** (references AnimationClip), **Blend** (weighted blend of children), **AdditiveBlend** (additive composition)
- `AnimationMask`: Bitfield for per-bone/target application control
- Evaluated bottom-up from root

#### Entity Binding (ECS Pattern)

- `AnimationPlayer`: Component on the controlling entity
- `AnimationGraphHandle`: Handle to AnimationGraph asset (same entity)
- `AnimatedBy`: Component on animated target entities, references the AnimationPlayer entity
- `AnimationTargetId`: UUID-based, generated from hierarchy path names

```rust
// Typical setup
let (graph, animation_index) = AnimationGraph::from_clip(animations.add(clip));
let mut player = AnimationPlayer::default();
player.play(animation_index).repeat();
commands.spawn((AnimationGraphHandle(graphs.add(graph)), player));
```

#### Transitions

- `AnimationTransitions` component manages fade-out based transitions
- `main_animation`: Currently active animation
- `transitions: Vec<AnimationTransition>`: Fading-out animations
- `weight_decline_per_sec`: Weight decay rate per second

#### Timeline

- No dedicated Timeline system
- `AnimationClip.events` field supports time-based event firing
- `add_event_to_target()` for target-specific events
- Third-party crates (e.g., `bevy_animation_graph`) for cinematic sequencing

---

### Game Engine Comparison Table

| Aspect | Unity | Unreal Engine | Godot | Bevy (Rust) |
|---|---|---|---|---|
| Clip Type | `AnimationClip` (Asset) | `UAnimSequence` (Asset) | `Animation` (Resource) | `AnimationClip` (Asset) |
| Control Pattern | State Machine | Graph + State Machine | Data/Control Split | DAG Graph |
| State Machine | Central | Inside AnimGraph | StateMachine Node | None |
| Blend Tree | BlendTree | BlendSpace 1D/2D | BlendSpace, BlendTree | Blend/AdditiveBlend nodes |
| Binding | Animator Component | SkeletalMeshComponent + AnimInstance | NodePath | ECS Components |
| Transitions | Crossfade + Params | TransitionRule + Inertialization | Crossfade + travel(A*) | Fade-out (weight decay) |
| Layers/Masks | Layers + Avatar Mask | Layers + per-bone Override | Blend node combinations | AnimationMask (bitfield) |
| Montage Equivalent | None (Timeline) | AnimMontage (Section/Slot) | None | None |
| Timeline | Timeline (Playable API) | Sequencer | Built into AnimationPlayer | None (events only) |
| Dynamic Graph | Playable API | Linked Anim Layers | Code AnimTree manipulation | Graph node add/remove |
| Property Generality | Any property | Primarily Skeletal Transform | Almost all node properties | AnimatableProperty trait |

---

## Part 2: DCC Tools (Digital Content Creation)

### 2.1 Blender -- Action, NLA Editor, Data-Blocks

#### Internal Data Architecture

- **DNA (Structure DNA)**: Binary struct descriptions embedded in `.blend` files. `makesdna` generates them at compile time. Enables backward compatibility (files from the 1990s still open).
- **RNA (Runtime Navigable Access)**: Introspective API layer mapping DNA to high-level properties. Used by UI, scripting, and animation system.

#### F-Curves and Actions

**F-Curves** are the fundamental animation unit:
- Each F-Curve contains keyframes for a single property channel
- Stores an RNA path (e.g., `pose.bones['Arm'].rotation_quaternion`) and `array_index` (0=X, 1=Y, 2=Z, 3=W)

**Actions** are generic containers for F-Curves:
- Can be attached to any data-block with matching RNA paths
- Portable and reusable across objects
- Action Groups organize F-Curves (typically by bone name)

#### Layered Action System (Blender 4.4+)

Three top-level arrays in an Action:
1. **Slots**: Labels that earmark animation data. Combined ID type + name is unique within an Action
2. **StripData**: Contains actual F-Curves. Multiple Strips can reference the same StripData (instancing)
3. **Layers**: Each Layer has an array of Strips that map StripData onto the Layer

#### NLA (Non-Linear Animation) System

- **Tracks**: Layering system, evaluated bottom-first, top-last
- **Strips**: Containers referencing Actions with timing/speed/influence/blend controls
- **Evaluation**: Nested function composition: `f_N(f_{N-1}(... f_1(defaults, strip_1), ...))`
- **Push Down**: Moves active action into a new muted NLA track
- **Stashing**: Stores unused actions as muted strips to prevent data loss
- **Tweak Mode**: Edit a strip's action in Action Editor while seeing NLA context

Internal evaluation functions (`anim_sys.cc`):
- `nla_blend_value()`, `nla_combine_value()`, `nla_combine_quaternion()`
- `animsys_append_tweaked_strip()`

#### Editors

- **Dope Sheet**: Keyframe timing overview
- **Action Editor**: F-Curves of active Action (Dope Sheet mode)
- **Graph Editor**: F-Curve values with tangent handles
- **NLA Editor**: Tracks, strips, and blending relationships

#### Action vs NLA Strip Relationship

- NLA Strip is a **container** referencing an Action
- Strip controls: timing, speed, influence, blend mode
- Action holds: actual keyframe data (F-Curves)
- One Action can be instanced across many strips
- Editing in Tweak Mode affects all strips referencing the same Action

#### Export Pipeline (glTF)

- **Actions Mode (Default)**: Exports all actions bound to a single armature
- **NLA Tracks Mode**: Each NLA Track becomes an independent glTF animation
  - Tracks with same name on different objects merge into one glTF animation
- **Merge Options**: By Action (using slot), by NLA Track Name, or no merge

Recommended workflow for game engines:
1. Apply all transforms on armature and mesh
2. Give each action a Fake User
3. Push Down each action to create NLA strips
4. Rename strips to desired animation names
5. Export with "NLA Tracks" mode

---

### 2.2 Maya -- Dependency Graph + Animation Layers

#### Internal Architecture: Dependency Graph (DG)

Animation curves are DG nodes connecting to animatable attributes. Eight animCurve types:

| Type | Input | Output |
|---|---|---|
| `animCurveTA` | Time | Angle |
| `animCurveTL` | Time | Linear (distance) |
| `animCurveTT` | Time | Time |
| `animCurveTU` | Time | Unitless |
| `animCurveUA` | Unitless | Angle |
| `animCurveUL` | Unitless | Linear |
| `animCurveUT` | Unitless | Time |
| `animCurveUU` | Unitless | Unitless |

- **Implicit Time Connection**: Time-based types (TA, TL, TT, TU) connect to DG time node implicitly
- **Static Optimization**: If all key values are equal and tangent Y-components are zero, evaluation is skipped
- **Dirty Propagation**: Lazy evaluation via dirty flags, cascading through dependencies

#### Animation Layers

- **Additive**: Layer values added to preceding layers (e.g., translateX=3 + translateX=2 = 5)
- **Override**: Layer values replace preceding layers for shared attributes
- **Override-Passthrough**: Override with adjustable opacity
- Each layer creates blend nodes in the DG
- Top-down priority for override layers
- Layers can be parented for organization
- Auto-key places keyframes on the last active layer

#### Time Editor vs Trax Editor

**Trax Editor (Legacy)**:
- Requires Character Sets (named attribute collections)
- Creates clips arranged on a non-linear timeline

**Time Editor (Modern)**:
- Works with any animated attribute (no Character Set required)
- Time Warp, Speed Curve, clip trimming/scaling/looping/splitting/grouping/crossfading
- Audio clip editing
- Multi-take FBX import support

#### Export Pipeline (FBX)

- Control rigs contain Maya-specific nodes (constraints, expressions, IK) incompatible with FBX
- **Baking required**: Transfer animation to joints and blendshapes only
- Manual baking (`Edit > Keys > Bake Simulation`) recommended over auto-bake
- Animation layers should be baked to base layer before export
- **Game Exporter**: Supports multiple animation clips with frame ranges for batch export
- Namespace matching between animation and skeleton is critical

---

### 2.3 3ds Max -- Controller Architecture + Motion Mixer

#### Controller System

Controllers are plug-ins handling all animation:
- **Single-Parameter Controllers**: One parameter (even multi-component like RGB)
- **Compound Controllers**: Multiple sub-controllers
  - **PRS (Position/Rotation/Scale)**: Default Transform controller
  - **Euler XYZ Rotation**: Per-axis Bezier tangent control
  - **List Controller**: Multiple controllers with weights
  - **Transform Script**: Programmable controller

Parameters don't receive a controller until animated. When a parameter changes at any non-zero frame with Auto Key enabled, a default controller is assigned.

#### Track View

- **Curve Editor**: Function curves with tangent handles (Bezier, TCB, Linear)
- **Dope Sheet**: Keyframe timing as blocks
- Euler rotation: Creating a key on one axis creates keys on all axes (locked)

#### Motion Mixer

Structure:
- **Trackgroups**: Collections of tracks, filterable by body parts
- **Tracks**: Weight curves controlling contribution within trackgroup
- **Clips**: Motion files (BIP, XAF) placed on tracks

Capabilities: Cross-fade, trim, time warp, weight curves for gradual blending

Limitation: To edit mixer animation with Curve Editor, you must create a mixdown and exit mixer mode first. Euler tangents become quaternion in mixdown.

#### Export Pipeline (FBX)

- **Key limitation**: Only one animation track per object per FBX file
- Workarounds:
  1. Sequential arrangement on single timeline (frame-range slicing)
  2. Separate FBX per animation (recommended)
  3. MotionBuilder passthrough for multi-take FBX

---

### 2.4 Houdini -- CHOP + KineFX Procedural Animation

#### CHOP (Channel Operator) Architecture

- Channels are sequences of numbers representing values over time
- CHOPs exist in node networks, connected in DAGs
- **Key CHOP Nodes**: Wave, Math, Filter, Resample, Trail, Channel
- **Time Slicing**: Optimization calculating only current frame range
- **Motion Effects**: CHOP networks that override parameter values procedurally
- **External I/O**: MIDI, raw files, TCP, audio devices, mouse cursor

#### KineFX Framework

Operates at the geometry (SOP) level:
- Joints = points with name and transform attributes
- Hierarchy = point connectivity
- Clear separation: Rigging (control setup) vs Deformation (skin movement)
- **Capture**: Static binding of mesh to joints (weight attributes)
- **Deformation**: Dynamic skin movement based on joints and bind pose
- **APEX**: Graph evaluation engine underneath KineFX
- **Motion Clips**: Freeze all frames into static geometry, apply geometric operations, convert back

#### Export Pipeline

- **FBX**: ROP FBX Animation Output node, can update existing FBX files
- **Vertex Animation**: Baked texture approaches for procedural effects (Vertex Animation Textures)
- **Import**: FBX, glTF, USD via dedicated SOP nodes
- For Unreal: Left-handed Z Up convention (Up=+Z, Front=-Y, Cross=-X)

---

### 2.5 Cinema 4D -- Motion System + Tags

#### Motion System Architecture

- **Motion System Tag**: Applied to an object, affects entire hierarchy below
- Nested Motion System tags within affected hierarchy are not allowed
- Internal structure:
  1. Motion System tag has NLA branch containing animation layers
  2. Each layer has its own NLA branch holding Motion Sources
  3. Original keyframe data lives inside nested branches

#### Motion Clips and Timeline

- Create via: Select animated object > Animate > Add Motion Clip
- **Motion Mode**: Third icon in timeline (next to Keyframe and F-Curve modes)
- Clip operations: Speed up/slow down by dragging ends, overlap for crossfade, loop, trim, fade
- **Motion Source Presets**: Save clips to Content Browser for reuse

#### Animation Without Keyframes

- Effectors (MoGraph)
- Tags and Expressions
- XPresso (node-based visual programming)
- Python scripting

#### Export Pipeline

- **FBX**: Native support. IK must be baked before export
- **glTF**: Supports TRS, PLA (Point Level Animation), Skin Animations, Pose Morph Targets
- **Limitations**: Cannot export combined animation layers, or PLA with pose morph targets
- Expressions, dynamics, MoGraph, generators must be baked before export

---

### DCC Tool Comparison Table

| Aspect | Blender | Maya | 3ds Max | Houdini | Cinema 4D |
|---|---|---|---|---|---|
| Clip Unit | Action (F-Curves) | AnimCurve nodes (DG) | Controllers | CHOP channels | Motion Clips |
| NLA System | NLA Editor | Time Editor | Motion Mixer | CHOP networks | Motion System |
| Layer System | NLA Tracks (4.4: Layers) | Animation Layers | List Controller | CHOP layers | Motion Layers |
| Blend Modes | Replace, Combine, Add | Additive, Override | Weight curves | Signal processing | Crossfade, Override |
| Procedural | Limited (drivers) | Expressions, MEL | MAXScript | Full procedural (CHOPs) | XPresso, MoGraph |
| glTF Export | Actions/NLA Tracks mode | Via FBX conversion | Via FBX conversion | Dedicated SOP node | Native support |
| FBX Export | Native | Native + Game Exporter | Native (1 track limit) | ROP FBX node | Native |
| Multi-clip Export | NLA Track names | Game Exporter takes | Separate files | Separate files | Separate files |

---

## Part 3: Cross-Cutting Analysis

### Destructive vs Non-Destructive Editing

| Tool | Non-Destructive Features | Destructive Export Step |
|---|---|---|
| Blender | NLA strips, layered actions (4.4+), stashing | Bake for glTF/FBX (optional sampling) |
| Maya | Animation layers, Time Editor clips | Bake layers + control rig for FBX |
| 3ds Max | Motion Mixer tracks, animation layers | Mixdown for curve editing, FBX bake |
| Houdini | CHOP networks, KineFX motion clips, procedural rigs | Bake procedural animation for FBX |
| Cinema 4D | Motion System clips, NLA blending | Bake expressions/MoGraph for FBX/glTF |

**Key insight**: DCC tools preserve rich non-destructive editing data internally, but game engine export is almost always destructive -- converting to flat keyframes for efficient runtime evaluation.

### Action Concept (Blender) in Detail

1. **Actions are data-blocks**: Exist independently of objects, shareable
2. **RNA path references**: Portable across objects with matching paths
3. **Action:NLA Strip = Content:Container**: Strip controls timing/speed/influence, Action holds keyframes
4. **Push Down**: Active action -> new muted NLA track (primary NLA workflow)
5. **Stashing**: Dormant action storage as muted strips

### Maya Layers vs Game Engine Layers

| Aspect | Maya (Authoring) | Game Engine (Runtime) |
|---|---|---|
| Driven by | Artist manual weight/mode settings | Game logic, parameters, state machines |
| Blend modes | Additive, Override, Override-Passthrough | BlendSpaces, Montages, Layered Rigs |
| Output | Single baked result for export | Real-time computed blends |
| Purpose | Non-destructive animation refinement | Responsive gameplay animation |

---

## Part 4: Implications for This Project

### Recommended Architecture Pattern

Given this project's existing ECS architecture and Rust language, **Bevy's pattern** is the most applicable:

1. **AnimationClip = Asset**: Already implemented. Maintain this approach
2. **AnimationGraph (DAG)**: Use Blend/AdditiveBlend nodes instead of a state machine. Incrementally extensible
3. **AnimationPlayer = Component**: Attach to entities for playback state management
4. **AnimationTransitions**: Fade-out based transitions via `weight_decline_per_sec`
5. **Timeline**: Defer dedicated Timeline system. Focus on AnimationGraph + AnimationPlayer first

### Godot's Data/Control Separation

Also applicable: the existing `AnimationPlayback` (resource) as control layer and `AnimationClip` (asset) as data layer already follows this pattern.

### DCC Tool Export Considerations

- glTF import from Blender: Support both per-Action and per-NLA-Track animation naming
- FBX import from Maya: Handle baked animation data with potential namespace prefixes
- Multiple animation clips per model file: Essential for practical workflow

---

## Part 5: Pixar Presto -- Studio-Grade Proprietary Animation System

### 5.1 History and Evolution

#### Before Presto: Marionette (Menv)

- **1988**: Eben Ostby and Bill Reeves developed **Menv (modeling environment)**. First used on the Oscar-winning short film "Tin Toy". Steve Jobs named it "Marionette" externally
- **1995**: "Toy Story" produced with Menv. Each shot was described as a single linear program file
- **1998**: "A Bug's Life" introduced referencing, layering, editing, and variations. Finite state machine crowd animation was also added
- **2004**: Marionette's organically evolved design became an impediment. Composition functionality was scattered across 3 different formats and "composition engines"

#### Birth of Presto

- **2005**: Ed Catmull announced building a new animation system from scratch. Rob Jensen led the development
- **2008**: Named after Doug Sweetland's short film "Presto"
- **2012**: **"Brave"** was the first feature film produced with Presto. All subsequent Pixar films use Presto
- **2018**: Received the Academy Scientific and Technical Achievement Award
  - **Rob Jensen**: Foundation design and ongoing development
  - **Thomas Hahn**: Animation toolset
  - **George ElKoura, Adam Woodbury, Dirk Van Gelder**: High-performance execution engine
- **2025**: Walt Disney Animation Studios adopted Presto for "Zootopia 2" (partial migration mid-production)

### 5.2 Architecture

#### Core Components

```
Presto Architecture
+-- Execution Engine (C++)
|     +-- Node network evaluation
|     +-- Dataflow processing
|     +-- Multithreaded rig solving
|
+-- Composition Engine
|     +-- Referencing, overrides, variations, inheritance
|     +-- Single ASCII format
|     +-- -> Became the core of USD
|
+-- Rig Solver
|     +-- Compiles high-level rig objects to optimized data structures
|     +-- Point posing and scalar field deformations
|     +-- Multi-level parallelism:
|           +-- Node-level
|           +-- Branch-level
|           +-- Model-level
|           +-- Frame-level (background multi-frame processing)
|
+-- Viewport (Hydra / Storm / RenderMan)
      +-- Real-time subdivision surfaces (OpenSubdiv)
      +-- Displacement, shadows, AO, DOF, PBR GLSL shading
      +-- Switchable render delegates (OpenGL <-> RenderMan)
```

#### Composition Engine -- The Heart of Presto (and USD)

The composition engine is Presto's most fundamental technology:
- Handles referencing, overrides, variations, and inheritance
- Operates at every granularity: single mesh to entire environments and shots
- Encoded in a single ASCII format
- **This composition engine directly became the core of USD (Universal Scene Description)**

Sebastian Grassia (USD project lead): "USD is the marriage of Presto's composition engine with a lazy-access, time-sampled data model, enhanced top-to-bottom for scalability and effective use of multi-core systems."

#### Relationship with Maya

- Presto was built in cooperation with Autodesk Maya ("extension of Maya")
- UI resembles Maya/3ds Max but with custom workflow innovations
- Modeling and initial rigging stages may still use Maya

### 5.3 Key Technical Features

| Feature | Description |
|---|---|
| Full-fidelity real-time preview | OpenSubdiv enables subdivision surfaces "within 1 pixel of final render" in real-time |
| Real-time viewport rendering | Displacement, shadows, AO, DOF, PBR GLSL shading |
| Collaborative workflow | USD-based layer editing allows multiple artists on the same scene simultaneously |
| Crowd system (PCF) | Generation and control of tens of thousands of agents |
| Sketch to Pose | Draw strokes to set multi-joint poses at once |
| ML Posing | Neural IK for natural pose generation |
| Invertible Rig | All rig components execute bidirectionally |
| Interactive Motion Blur | Real-time motion blur in viewport |

#### Presto vs Commercial DCC Tools

| Aspect | Presto | Commercial Tools (Maya etc.) |
|---|---|---|
| Target | Optimized for Pixar's pipeline | General purpose |
| Real-time | Full scene with fur, lighting, shadows | Limited |
| Collaboration | USD-based concurrent editing | Limited |
| Rig evaluation | Multithreaded parallel execution | Tool-dependent |
| Customization | Source-code level modifications | Plugin API only |
| Stability | "Like a race car -- sometimes crashes" | More stable |

### 5.4 Animation Workflow

#### Pixar's Pipeline

```
Story -> Layout -> Animation -> Simulation -> Lighting -> Rendering
              |                                    |
           Presto                              RenderMan
              |
          USD (data exchange foundation for all stages)
```

#### Animator's Work

1. **Scene-context work**: Animators use full-resolution geometry and advanced rig controls interactively within scene context
2. **Real-time feedback**: Changes reflected immediately in viewport
3. **Render delegate switching**: Dynamic switching between OpenGL (Storm) and RenderMan in viewport
4. **Layer-based editing**: Edit in own layer without breaking other artists' work

#### System Requirements (circa 2016)

- **OS**: Linux (stability and rendering speed)
- **Hardware**: Multi-core Intel CPU (16+ cores), 64GB+ RAM, NVIDIA Quadro professional GPU

### 5.5 USD -- Born from Presto

USD is positioned as the **4th generation** of Pixar's "composed scene description":

1. **Gen 1**: "Toy Story" era -- single linear files
2. **Gen 2**: Marionette/Menv -- referencing and layering ("A Bug's Life" onward)
3. **Gen 3**: Presto's unified composition engine ("Brave" onward)
4. **Gen 4**: **USD** -- fusion of Presto's composition engine with TidScene's lazy-access, time-sampled data model

Key milestones:
- 2012: USD project started
- 2016: Open-sourced under Apache 2.0 license
- "Finding Dory" was the first film with USD in production
- 2023: Alliance for OpenUSD established with major tech companies
- Pixar is working on open-sourcing Presto's "execution engine and in-memory scene representation" into OpenUSD

### 5.6 Pixar Open-Source Ecosystem

| Project | Role | Relationship to Presto |
|---|---|---|
| **OpenUSD** | Scene description & composition | Presto's composition engine, open-sourced |
| **OpenSubdiv** | Subdivision surface evaluation | Enables real-time limit surfaces in Presto (3ms vs 100ms with Maya) |
| **Hydra** | Imaging framework (render delegates) | Presto's viewport backend; Storm (OpenGL) and RenderMan delegates |
| **RenderMan** | Production renderer | Final frame rendering; interactive preview in Presto via Hydra |
| **OpenEXR** | HDR image format | Output format for rendered frames |
| **Alembic** | Geometry cache exchange | Baked geometry data exchange |

#### Hydra Architecture in Presto

```
Presto Viewport
    |
    v
Hydra (Imaging Framework)
    |
    +-- Storm (OpenGL/Vulkan delegate) -- real-time preview
    +-- RenderMan delegate -- production-quality preview
    +-- HdEmbree -- CPU raytracing reference
```

- Hydra is migrating from OpenGL to Vulkan via Hgi (Hydra Graphics Interface) abstraction layer
- OpenUSD 24.08 added experimental Vulkan backend support (Pixar, Autodesk, Adobe collaboration)

#### RenderMan Integration

- **RenderMan 22+**: Interactive path tracing in Presto viewport
- **RenderMan 27**: XPU (extreme processing unit) GPU-accelerated rendering as primary; RIS (CPU) renderer in maintenance mode

### 5.7 Disney Animation Studios

Disney Animation is Pixar's sister studio with its own tools:

| Tool | Description | First Used |
|---|---|---|
| **Hyperion** | Physics-based path tracer with sorted ray batch architecture | "Big Hero 6" (2014) |
| **Meander** | 2D vector/raster hybrid animation (strokes following 3D motion) | "Paperman", "Feast" |
| **Matterhorn** | Snow simulation (Material Point Method) | "Frozen" |
| **Quicksilver** | Hair system | "Moana" |
| **PhysGrid** | Muscle/soft tissue simulation | Various |
| **Tonic** | Stylized hairstyle construction | Various |

Disney adopted Presto for "Zootopia 2" (2025), marking convergence between the two studios' toolchains. The migration enabled cross-department dialogue and synchronous troubleshooting.

### 5.8 Technical Publications

| Year | Title | Venue |
|---|---|---|
| 2013 | Presto Execution System | Internal/SIGGRAPH |
| 2015 | Sketch to Pose in Pixar's Presto Animation System | SIGGRAPH 2015 Talks |
| 2016 | Real-Time Graphics in Pixar Film Production | SIGGRAPH 2016 Real-Time Live! |
| 2019 | Multithreading in Pixar's Animation Tools | SIGGRAPH Asia 2019 Course |
| 2020 | Next-Gen Rendering Technology at Pixar | GTC Digital 2020 |
| 2022 | Presto Crowds Foundations (PCF) | SIGGRAPH 2022 |
| 2023 | Pose and Skeleton-aware Neural IK | SIGGRAPH Asia 2023 |
| 2024 | What's New in Pixar's Presto: ML Posing, Invertible Rigs, and Interactive Motion Blur | SIGGRAPH Asia 2024 Real-Time Live! |

---

## Part 6: This Project -- Proposed Animation Architecture

### 6.1 Project Goals and Constraints

This project is a **Vulkan-based rendering engine with animation editing capabilities**, not a game engine.

**Goals**:
- Edit animations (keyframe editing, curve editing)
- Save animation clips to files
- See results via real-time Vulkan rendering
- Models and rigs come from Maya (glTF/FBX import)

**Constraints**:
- ECS architecture is mandatory (data/behavior separation)
- No state machines or blend graphs (too complex, not needed for editor)
- No game engine build system
- Simple, programmer-friendly data management

### 6.2 Current State Analysis

The project already has a substantial animation foundation:

| Layer | Current Implementation | Status |
|---|---|---|
| Data | `AnimationClip` / `EditableAnimationClip` | Working |
| Editing | CurveEditor / Timeline UI | Working |
| Persistence | RON format save/load | Working |
| Playback | `AnimationPlayback` (global Resource) | Working |
| Evaluation | `playback_prepare_animations` -> GPU | Working |

**Structural issues in current design**:

1. **Playback is global**: `AnimationPlayback` exists as a single Resource -- only one clip can play for the entire scene
2. **Entity-clip binding is implicit**: Which entity plays which clip depends solely on `AnimationPlayback.current_clip_id`
3. **`AnimationState` component is vestigial**: Defined but the actual evaluation path uses the `AnimationPlayback` Resource
4. **`EditableClipManager` lives outside proper ECS flow**: Synchronization with `AnimationRegistry` is manual

### 6.3 Design Philosophy

```
Blender's "Data/Control Separation" + Bevy's "ECS Pattern"
= Simple Two-Layer Animation Design
```

- **No state machine**: Not a game engine, no runtime state transitions needed
- **No blend graph**: No complex DAG required at this stage
- **NLA-like clip arrangement**: Future consideration, start with 1-entity-1-clip direct playback

### 6.4 Proposed Architecture

#### Layer Diagram

```
+--------------------------------------------------+
|  Asset Layer (Data)                               |
|  +-------------------+  +----------------------+ |
|  | AnimationClip     |  | EditableAnimationClip| |
|  | (playback, light) |  | (editing, curves)    | |
|  +-------------------+  +----------------------+ |
|         ^ conversion            ^ conversion      |
|  +---------------------------------------------------+
|  | ClipLibrary (Resource)                             |
|  |   clips: HashMap<ClipId, AnimationClip>            |
|  |   editable: HashMap<ClipId, EditableAnimationClip> |
|  |   skeletons: Vec<Skeleton>                         |
|  |   clip_metadata: HashMap<ClipId, ClipMetadata>     |
|  +---------------------------------------------------+
+--------------------------------------------------+
|  Component Layer (Entity)                         |
|  +---------------------------------------------+ |
|  | Animator (Component)                         | |
|  |   active_clip_id: Option<ClipId>             | |
|  |   time: f32                                  | |
|  |   speed: f32                                 | |
|  |   playing: bool                              | |
|  |   looping: bool                              | |
|  |   skeleton_id: Option<SkeletonId>            | |
|  +---------------------------------------------+ |
+--------------------------------------------------+
|  System Layer (Evaluation)                        |
|  +---------------------------------------------+ |
|  | advance_animators(world, delta_time)         | |
|  | evaluate_animators(world, clip_library, ...) | |
|  |   -> query entities with Animator            | |
|  |   -> clip.sample(time, skeleton)             | |
|  |   -> apply skinning or node animation        | |
|  +---------------------------------------------+ |
+--------------------------------------------------+
|  Editor Layer (UI)                                |
|  +-----------+ +----------+ +---------+          |
|  | Timeline  | | Curve    | | Insp.   |          |
|  | Window    | | Editor   | | Animator|          |
|  +-----------+ +----------+ +---------+          |
+--------------------------------------------------+
```

#### Asset Layer: ClipLibrary

Unifies current `AnimationRegistry` + `EditableClipManager` into a single concept.

```
ClipLibrary (Resource)
+-- clips: HashMap<ClipId, AnimationClip>
+-- editable_clips: HashMap<ClipId, EditableAnimationClip>
+-- skeletons: Vec<Skeleton>
+-- dirty_clips: HashSet<ClipId>
+-- clip_metadata: HashMap<ClipId, ClipMetadata>

ClipMetadata
+-- name: String
+-- source_path: Option<String>  (original glTF path)
+-- duration: f32
+-- skeleton_id: Option<SkeletonId>
```

**Role**: Centralized clip lifecycle management. Clips exist as "library items", not owned by entities. Same philosophy as Blender's Actions.

#### Component Layer: Animator

Evolves current `AnimationState` and absorbs `AnimationPlayback` Resource's role.

```
Animator (Component)
+-- active_clip_id: Option<ClipId>
+-- time: f32
+-- speed: f32
+-- playing: bool
+-- looping: bool
+-- skeleton_id: Option<SkeletonId>
```

**Entity composition example**:

```
Stickman Entity
+-- Name("stickman")
+-- Transform { ... }
+-- Visible(true)
+-- MeshRef { mesh_asset_id, object_index }
+-- Animator {                          <- NEW
|     active_clip_id: Some(clip_3),
|     time: 1.5,
|     playing: true,
|     ...
|   }
+-- EditorDisplay { icon: Model }
```

**Benefit**: Each entity can independently play its own animation. Multiple characters can coexist with independent playback.

#### System Layer: Evaluation Functions

```rust
// Called every frame -- advance time for all animators
pub fn advance_animators(world: &mut World, delta_time: f32)

// Evaluate animation poses for all entities with Animator
pub fn evaluate_animators(
    world: &World,
    clip_library: &ClipLibrary,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
) -> Vec<usize>  // returns updated mesh indices
```

#### Editor Layer: Timeline <-> Animator Linkage

```
Timeline controls the selected entity's Animator:

Hierarchy: select Entity
  -> Inspector shows Animator section
  -> Timeline displays Animator.active_clip_id's clip
  -> Play button -> Animator.playing = true
  -> Scrubber -> Animator.time = scrub_time
  -> Keyframe edit -> ClipLibrary's editable_clip updated
```

**TimelineState changes**:

```
TimelineState (Resource, UI state only)
+-- target_entity: Option<Entity>      <- controlled entity
+-- zoom_level: f32
+-- scroll_offset: f32
+-- selected_keyframes: HashSet<...>
+-- expanded_tracks: HashSet<BoneId>
+-- show_translation / rotation / scale: bool
    (playing, time, speed move to Animator)
```

### 6.5 Data Flow

```
[Model Load (glTF/FBX)]
    |
    +-- Skeleton -> ClipLibrary.skeletons
    +-- AnimationClip(s) -> ClipLibrary.clips
    +-- EditableAnimationClip(s) -> ClipLibrary.editable_clips
    |
    +-- Entity spawn
         +-- MeshRef, Transform, ...
         +-- Animator { active_clip_id, skeleton_id, ... }

[Every Frame]
    |
    +-- UI Input -> UIEvent
    |     +-- TimelinePlay -> animator.playing = true
    |     +-- TimelineSetTime -> animator.time = t
    |     +-- TimelineMoveKeyframe -> clip_library.edit(...)
    |     +-- TimelineSelectClip -> animator.active_clip_id = id
    |
    +-- advance_animators(world, dt)
    |     +-- animator.time += dt * speed
    |
    +-- sync_dirty_clips(clip_library)
    |     +-- EditableClip -> AnimationClip conversion
    |
    +-- evaluate_animators(world, clip_library, graphics)
    |     +-- clip.sample(time, skeleton)
    |     +-- apply_skinning() or node_animation()
    |     +-- -> updated_mesh_indices
    |
    +-- upload_to_gpu(updated_meshes)

[Save]
    +-- clip_library.save_clip(clip_id, path)
         -> RON file
```

### 6.6 Migration Plan (Initial Draft -- Superseded by 6.8)

Initial migration plan (kept for reference):

| Phase | Change | Files Affected |
|---|---|---|
| **Phase 1** | Define `Animator` component, spawn with model load | `world.rs`, `scene_model.rs` |
| **Phase 2** | Migrate playback state from `AnimationPlayback` to `Animator` | `animation_playback_systems.rs`, `timeline_systems.rs` |
| **Phase 3** | Remove playback state from `TimelineState`, add `target_entity` | `timeline_state.rs`, `timeline_window.rs` |
| **Phase 4** | Unify `AnimationRegistry` + `EditableClipManager` -> `ClipLibrary` | `resource/`, `manager.rs` |
| **Phase 5** | Add Animator section to Inspector UI | `inspector_systems.rs`, `inspector_window.rs` |

**This plan was superseded** -- see Section 6.8 for the revised migration plan that addresses ECS compliance issues found during review.

### 6.7 Mapping to Blender / Presto

| This Project | Blender | Presto |
|---|---|---|
| `AnimationClip` (Asset) | Action | Animation Data |
| `EditableAnimationClip` | Action (F-Curves) | Internal curve representation |
| `ClipLibrary` (Resource) | Data-blocks | USD Asset Database |
| `Animator` (Component) | Object's active Action | Entity animation state |
| Timeline Window | NLA Editor / Dope Sheet | Timeline |
| Curve Editor | Graph Editor | Curve Editor |
| `evaluate_animators` (System) | Dependency Graph evaluation | Execution Engine |

The key similarity to Presto is the **clear separation between "clips as data" and "animator on entity"**. This matches Blender's pattern where Action = data, Object's active Action = binding.

### 6.8 Revised Migration Plan (ECS-Compliant)

The initial migration plan (6.6) had three structural issues discovered during ECS compliance review.

#### Issue 1: Evaluation Path Bypasses Entities

The current `playback_prepare_animations` operates directly on `graphics.meshes[i]` array indices:

```
AnimationPlayback (Resource, global)
    -> graphics.meshes[i]  (direct array index access)
    -> skeleton_id obtained from graphics.meshes.first()
```

**No Entity is involved in the evaluation path.** Creating an `Animator` component without changing the evaluation path would result in the same problem as the current vestigial `AnimationState` -- a component that exists but is never read by the actual animation systems.

#### Issue 2: model_path in AnimationPlayback

`AnimationPlayback.model_path` is used for animation type detection:

```rust
let is_gltf = model_path.ends_with(".glb");
let has_node_animation = (is_gltf || is_fbx) && !model_state.has_skinned_meshes;
```

This is **model metadata**, not playback state. It does not belong in `Animator`, but removing it from `AnimationPlayback` breaks the evaluation path.

#### Issue 3: MorphAnimation is a Separate System

`MorphAnimationSystem` has a fundamentally different data structure from skeletal animation. It stores per-mesh morph targets and weight arrays, not per-bone transform channels. It cannot be trivially unified into a `ClipLibrary` alongside `AnimationClip`.

#### Revised Phase Plan

| Phase | Change | Risk | Key Files |
|---|---|---|---|
| **Phase 1** | Define `Animator` component (data only), attach to entities on model load | None | `world.rs`, `scene_model.rs` |
| **Phase 2** | Bridge: sync selected entity's `Animator` -> `AnimationPlayback` | None | New system function, `animation_phase.rs` |
| **Phase 3** | Move `model_path` from `AnimationPlayback` to `ModelState`, add `AnimationType` cache | Low | `animation_playback.rs`, `graphics.rs`, `animation_playback_systems.rs` |
| **Phase 4** | Remove playback state from `TimelineState`, add `target_entity`; Timeline operates on Animator directly | Low | `timeline_state.rs`, `timeline_window.rs`, `timeline_systems.rs` |
| **Phase 5** | Replace evaluation path: Entity-based `evaluate_animators` replaces `playback_prepare_animations` | **High** | `animation_playback_systems.rs`, `animation_phase.rs` |
| **Phase 6** | Unify `AnimationRegistry` + `EditableClipManager` -> `ClipLibrary` (morph kept as separate field) | Medium | `resource/graphics.rs`, `editable/manager.rs` |
| **Phase 7** | Add Animator section to Inspector UI | None | `inspector_systems.rs`, `inspector_window.rs` |

#### Phase 1: Animator Component (Data Only)

Define component and attach at model load. Existing evaluation path unchanged.

```rust
#[derive(Clone, Debug)]
pub struct Animator {
    pub active_clip_id: Option<AnimationClipId>,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
    pub looping: bool,
}
```

`AnimationPlayback` Resource remains fully functional at this stage.

#### Phase 2: Bridge (Animator -> AnimationPlayback Sync)

Create a system that copies the selected entity's `Animator` state into the existing `AnimationPlayback` Resource. This allows the entire existing evaluation pipeline to continue working without modification.

```rust
pub fn sync_animator_to_playback(
    world: &World,
    target_entity: Option<Entity>,
    playback: &mut AnimationPlayback,
) {
    if let Some(entity) = target_entity {
        if let Some(animator) = world.get_component::<Animator>(entity) {
            playback.time = animator.time;
            playback.playing = animator.playing;
            playback.speed = animator.speed;
            playback.looping = animator.looping;
            playback.current_clip_id = animator.active_clip_id;
        }
    }
}
```

After evaluation, sync back:

```rust
pub fn sync_playback_to_animator(
    world: &mut World,
    target_entity: Option<Entity>,
    playback: &AnimationPlayback,
) {
    if let Some(entity) = target_entity {
        if let Some(animator) = world.get_component_mut::<Animator>(entity) {
            animator.time = playback.time;
        }
    }
}
```

**Key insight**: The existing `playback_prepare_animations` function is untouched. The bridge pattern ensures zero breakage.

#### Phase 3: model_path to ModelState

Move model metadata out of `AnimationPlayback`:

```
ModelState (Resource) -- revised
+-- has_skinned_meshes: bool
+-- node_animation_scale: f32
+-- model_path: String                  <- moved from AnimationPlayback
+-- animation_type: AnimationType       <- cached detection result

enum AnimationType {
    None,
    Skeletal,
    Node,
    Morph,
}
```

Update `playback_prepare_animations` signature to read `model_path` and `animation_type` from `ModelState` instead of `AnimationPlayback`. The `AnimationPlayback.model_path` field is then removed.

#### Phase 4: TimelineState Cleanup

Remove playback state from `TimelineState`. Timeline UI reads from / writes to the `Animator` component on `target_entity`.

```
TimelineState (Resource, UI state only) -- revised
+-- target_entity: Option<Entity>       <- replaces embedded playback state
+-- zoom_level: f32
+-- scroll_offset: f32
+-- selected_keyframes: HashSet<SelectedKeyframe>
+-- expanded_tracks: HashSet<BoneId>
+-- show_translation: bool
+-- show_rotation: bool
+-- show_scale: bool
```

Timeline events route through `Animator`:
- `TimelinePlay` -> set `animator.playing = true`
- `TimelineSetTime(t)` -> set `animator.time = t`
- `TimelineSelectClip(id)` -> set `animator.active_clip_id = Some(id)`

#### Phase 5: Entity-Based Evaluation (Critical Phase)

Replace `playback_prepare_animations` with a new function that traverses entities:

```rust
pub fn evaluate_animators(
    world: &World,
    clip_library: &ClipLibrary,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    model_state: &ModelState,
) -> Vec<usize> {
    let mut updated = Vec::new();

    // Query all entities with Animator + MeshRef
    for (entity, animator, mesh_ref) in query_animated_entities(world) {
        let clip = match animator.active_clip_id
            .and_then(|id| clip_library.clips.get(&id))
        {
            Some(c) => c,
            None => continue,
        };

        // Entity -> MeshRef -> graphics.meshes[index]
        let mesh_index = mesh_ref_to_graphics_index(mesh_ref, &assets);

        match model_state.animation_type {
            AnimationType::Skeletal => {
                // clip.sample() -> apply_skinning on graphics.meshes[mesh_index]
            }
            AnimationType::Node => {
                // clip.sample() -> node transform on nodes[]
            }
            AnimationType::Morph => {
                // morph evaluation on graphics.meshes[mesh_index]
            }
            AnimationType::None => {}
        }

        updated.push(mesh_index);
    }

    updated
}
```

**This is the highest-risk phase** because it replaces the core evaluation pipeline. The bridge from Phase 2 should be kept as a fallback during development.

#### Phase 6: ClipLibrary Unification

Merge `AnimationRegistry` + `EditableClipManager` into `ClipLibrary`. `MorphAnimationSystem` is kept as a separate field due to its fundamentally different data model.

```
ClipLibrary (Resource)
+-- clips: HashMap<ClipId, AnimationClip>
+-- editable_clips: HashMap<ClipId, EditableAnimationClip>
+-- skeletons: Vec<Skeleton>
+-- morph: MorphAnimationSystem         <- kept as separate structure
+-- dirty_clips: HashSet<ClipId>
+-- clip_metadata: HashMap<ClipId, ClipMetadata>

ClipMetadata
+-- name: String
+-- source_path: Option<String>
+-- duration: f32
+-- skeleton_id: Option<SkeletonId>
```

#### Phase 7: Inspector Animator Section

Add Animator information display to the Inspector window, similar to how Mesh/Material sections were added.

#### Comparison: Initial vs Revised Plan

| Aspect | Initial Plan (6.6) | Revised Plan (6.8) |
|---|---|---|
| AnimationPlayback handling | Replaced immediately in Phase 2 | Bridge in Phase 2, replaced in Phase 5 |
| Evaluation path | Unchanged (implicit) | Explicitly converted to Entity-based in Phase 5 |
| model_path | Not addressed | Moved to ModelState in Phase 3 |
| MorphAnimation | Merged into ClipLibrary | Kept as separate field within ClipLibrary |
| Breaking change risk | High at Phase 2 | Contained to Phase 5 only |
| Existing pipeline survival | Phases 1-3 only | Phases 1-4 (bridge keeps old path alive) |

---

## References

### Game Engines
- [Unity - Animator Controller](https://docs.unity3d.com/6000.2/Documentation/Manual/class-AnimatorController.html)
- [Unity - Playable API](https://docs.unity3d.com/Manual/Playables.html)
- [Unreal - Animation Blueprints](https://dev.epicgames.com/documentation/en-us/unreal-engine/animation-blueprints-in-unreal-engine)
- [Unreal - State Machines](https://dev.epicgames.com/documentation/en-us/unreal-engine/state-machines-in-unreal-engine)
- [Unreal - Animation Sequences](https://dev.epicgames.com/documentation/en-us/unreal-engine/animation-sequences-in-unreal-engine)
- [Godot - AnimationTree](https://docs.godotengine.org/en/stable/tutorials/animation/animation_tree.html)
- [Godot - AnimationPlayer](https://docs.godotengine.org/en/stable/classes/class_animationplayer.html)
- [Bevy - AnimationGraph](https://docs.rs/bevy/latest/bevy/prelude/struct.AnimationGraph.html)
- [Bevy - AnimationPlayer](https://docs.rs/bevy/latest/bevy/animation/struct.AnimationPlayer.html)
- [Bevy - bevy_animation source](https://github.com/bevyengine/bevy/tree/main/crates/bevy_animation/src)

### DCC Tools
- [Blender - NLA Editor](https://docs.blender.org/manual/en/latest/editors/nla/index.html)
- [Blender - Actions](https://docs.blender.org/manual/en/latest/animation/actions.html)
- [Blender - Animation 2025 (Layered Actions)](https://developer.blender.org/docs/features/animation/slotted-actions/)
- [Maya - Animation Layers](https://help.autodesk.com/view/MAYAUL/2025/ENU/?guid=GUID-79BEFF10-13D7-4E39-9573-D94AD40B5030)
- [Maya - Time Editor](https://help.autodesk.com/view/MAYAUL/2025/ENU/?guid=GUID-55E2E09C-6D24-4079-B0CC-B1E9D3D8F87E)
- [Maya - FBX Export for Games](https://help.autodesk.com/view/MAYAUL/2025/ENU/?guid=GUID-456BA184-B45A-4AEE-B6B4-C7B65B89E09A)
- [3ds Max - Motion Mixer](https://help.autodesk.com/view/3DSMAX/2025/ENU/?guid=GUID-6CC0B7A2-8B8B-4A7B-81AA-392F6E7A1C6F)
- [3ds Max - Animation Controllers](https://help.autodesk.com/view/3DSMAX/2025/ENU/?guid=GUID-2E99A5D0-D7AC-42B0-8D48-7672DB5E03D2)
- [Houdini - CHOPs](https://www.sidefx.com/docs/houdini/nodes/chop/index.html)
- [Houdini - KineFX](https://www.sidefx.com/docs/houdini/character/kinefx/index.html)
- [Cinema 4D - Motion System](https://help.maxon.net/c4d/en-us/Content/html/52080.html)

### Pixar Presto & Related
- [Presto - Pixar Animation Studios](https://www.pixar.com/presto)
- [Presto (animation software) - Wikipedia](https://en.wikipedia.org/wiki/Presto_(animation_software))
- [A rare peek at Presto, Pixar's secret weapon - Digital Trends](https://www.digitaltrends.com/computing/pixar-shows-software-at-gtc-2016/)
- [Introduction to USD - OpenUSD](https://openusd.org/release/intro.html)
- [Pixar's USD Pipeline - RenderMan](https://renderman.pixar.com/stories/pixars-usd-pipeline)
- [Scientific & Technical Awards 2018 - Oscars](https://www.oscars.org/sci-tech/ceremonies/2018)
- [Sketch to Pose in Pixar's Presto - ACM](https://dl.acm.org/doi/10.1145/2775280.2792583)
- [What's New in Pixar's Presto - SIGGRAPH Asia 2024](https://dl.acm.org/doi/10.1145/3681757.3697056)
- [Multithreading in Pixar's Animation Tools - SIGGRAPH Asia 2019](https://sa2019.siggraph.org/attend/courses/session/18/details/28)
- [Real-Time Graphics in Pixar Film Production - SIGGRAPH 2016](https://history.siggraph.org/wp-content/uploads/2022/06/2016-Realtime-Live-Gelder_Real-Time-Graphics-in-Pixar-Film-Production.pdf)
- [PCF Paper - Pixar Graphics](https://graphics.pixar.com/library/PCF/paper.pdf)
- [Pose and Skeleton-aware Neural IK - ACM](https://dl.acm.org/doi/10.1145/3610548.3618217)
- [OpenSubdiv - Pixar Graphics](https://graphics.pixar.com/opensubdiv/docs/intro.html)
- [Hydra SIGGRAPH 2019 - OpenUSD](https://openusd.org/files/Siggraph2019_Hydra.pdf)
- [Vulkan Support in Hydra - Khronos](https://www.khronos.org/blog/vulkan-support-added-to-openusd-and-pixars-hydra-storm-renderer)
- [Disney Animation Technology](https://disneyanimation.com/technology/)
- [Disney's Hyperion Renderer](https://disneyanimation.com/technology/hyperion/)
