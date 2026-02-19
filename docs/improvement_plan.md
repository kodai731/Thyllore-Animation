# Improvement Plan: DCC Tool Comparison & Gap Analysis

This document compares the current project with major DCC tools (Maya, MotionBuilder, Presto,
Blender, Houdini) and identifies improvement areas focused on **animation authoring** and
**rendering**.

---

## Current Project Feature Summary

| Category             | Status                                                            |
|----------------------|-------------------------------------------------------------------|
| Animation Types      | Skeletal, Node, Morph Target, Spring Bone                         |
| Curve Interpolation  | Linear, Bezier, Stepped                                           |
| Timeline             | Dope Sheet, Graph Editor                                          |
| Clip System          | Library, Instances, Groups, Blending (Override/Additive)          |
| Constraints          | IK, Aim, Parent, Position, Rotation, Scale                       |
| Import               | glTF, FBX                                                         |
| Export               | RON (internal format only)                                        |
| Undo/Redo            | Clip and Schedule edits                                           |
| ML Features          | Curve Copilot, Text-to-Motion                                     |
| Rendering            | Deferred + Ray Query Shadows, Bloom, DoF, Auto Exposure           |

---

## 1. File Export (Critical)

### Current State

- Import: glTF, FBX supported
- Export: Internal RON format only (not interchangeable with other tools)

### Industry Standard (Maya / MotionBuilder / Blender)

- FBX export with animation bake
- glTF/GLB export (Blender, Godot ecosystem)
- Alembic export for cached animation
- USD export (Pixar Presto, SideFX Houdini)

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P0       | FBX animation export         | Export edited animation keyframes applied to original skeleton as .fbx          |
| P0       | GLB animation export         | Export as .glb with embedded animation data                                    |
| P1       | Animation-only export        | Export animation data separately (without mesh) for retargeting workflows      |
| P1       | Bake before export           | Bake constraints and spring bones into keyframes before export                 |
| P2       | Alembic (.abc) export        | Point-cached animation export for VFX pipelines                               |
| P2       | USD export                   | Universal Scene Description for studio pipelines                              |

---

## 2. Graph Editor / Curve Editor

### Current State

- Bezier, Linear, Stepped interpolation
- Tangent handle editing (In/Out)
- Auto tangent
- Pan/Zoom
- Keyframe context menu
- ML-powered curve suggestion (Curve Copilot)

### Industry Standard

| Feature                        | Maya | MotionBuilder | Blender | This Project |
|--------------------------------|------|---------------|---------|--------------|
| Weighted tangents              | Yes  | Yes           | Yes     | No           |
| Tangent types (Auto/Spline/Flat/Plateau/Clamped) | Yes | Yes | Yes | Partial (Auto only) |
| Curve pre/post infinity (Cycle/Oscillate/Linear) | Yes | Yes | Yes | No |
| F-Curve modifiers (Noise/Cycles/Envelope) | -  | -             | Yes     | No           |
| Buffer curves (before/after comparison) | Yes | -            | -       | No           |
| Value snapping (grid snap for values) | Yes | Yes          | Yes     | No           |
| Box/region select keyframes    | Yes  | Yes           | Yes     | No           |
| Flatten/Align tangents batch   | Yes  | Yes           | Yes     | No           |
| Copy/paste curve shapes        | Yes  | Yes           | Yes     | No           |
| Normalize display              | Yes  | -             | Yes     | No           |
| Isolate selected curves        | Yes  | Yes           | Yes     | No           |

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P0       | Weighted tangents            | Allow independent length control of In/Out tangent handles                     |
| P0       | More tangent types           | Add Flat, Clamped, Plateau, Spline tangent presets                             |
| P1       | Pre/Post infinity modes      | Cycle, Oscillate, Linear, Constant for curve looping                           |
| P1       | Box select keyframes         | Drag rectangle to select multiple keyframes in graph editor                    |
| P1       | Value grid snap              | Snap keyframe values to grid lines (configurable step)                         |
| P2       | Buffer curves                | Show before/after overlay when editing curves                                  |
| P2       | Normalize curve display      | Normalize all visible curves to 0-1 range for comparison                       |
| P2       | Batch tangent operations     | Flatten, Align, Break tangents for multiple keyframes at once                  |
| P3       | F-Curve modifiers            | Procedural noise, cycle repeat modifiers on curves                             |

---

## 3. Timeline / Dope Sheet

### Current State

- Dope sheet with diamond keyframe markers
- Track expand/collapse per bone
- Transport controls (Play/Pause/Stop/Loop)
- Zoom and speed control
- Snap to frame/keyframe
- Clip track section (instance move, trim, mute, blend)
- Copy/Paste keyframes with mirror paste

### Industry Standard

| Feature                        | Maya | MotionBuilder | Blender | This Project |
|--------------------------------|------|---------------|---------|--------------|
| NLA / Nonlinear Animation Editor | Trax | Story | NLA Editor | Clip track (basic) |
| Animation layers (additive stacking) | Yes | Yes | Yes | Clip groups (partial) |
| Multi-row keyframe drag        | Yes  | Yes           | Yes     | No           |
| Range select (drag time range) | Yes  | Yes           | Yes     | No           |
| Markers / Bookmarks            | Yes  | Yes           | Yes     | No           |
| Audio track sync               | Yes  | Yes           | Yes     | No           |
| Playback range (in/out points) | Yes  | Yes           | Yes     | No           |
| Framerate display toggle (frames/seconds/timecode) | Yes | Yes | Yes | Seconds only |
| Keyframe coloring by type      | Yes  | -             | Yes     | Partial (property color) |
| Scale keyframes (stretch time) | Yes  | Yes           | Yes     | No           |
| Reverse keyframes              | Yes  | -             | Yes     | No           |
| Keyframe nudge (arrow keys)    | Yes  | Yes           | Yes     | No           |
| Multi-track drag in dope sheet | Yes  | Yes           | Yes     | No           |

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P0       | Keyframe range select        | Drag to select keyframes within a time range                                   |
| P0       | Multi-keyframe drag          | Drag multiple selected keyframes together on dope sheet                        |
| P1       | Time display mode toggle     | Switch between seconds / frames / timecode (SMPTE)                            |
| P1       | Playback range (in/out)      | Set loop region for partial playback                                           |
| P1       | Scale keyframes              | Stretch/compress selected keyframes in time                                    |
| P1       | Keyframe nudge (arrow keys)  | Move selected keyframes by 1 frame with arrow keys                            |
| P2       | Timeline markers             | Named bookmarks at specific times (e.g. "contact", "peak")                    |
| P2       | Reverse keyframes            | Reverse time order of selected keyframes                                       |
| P2       | Audio track                  | Load WAV/MP3 for lip-sync or timing reference                                 |
| P3       | NLA enhancement              | Transition blending, clip speed multiplier, clip looping                       |

---

## 4. Viewport & Visualization

### Current State

- Orbit/Pan/Zoom camera
- 9 debug view modes (Final, Position, Normal, Shadow, etc.)
- Bone gizmo (4 display styles: Stick/Octahedral/Box/Sphere)
- Grid display
- Light gizmo (draggable)
- Constraint gizmo visualization

### Industry Standard

| Feature                        | Maya | MotionBuilder | Blender | This Project |
|--------------------------------|------|---------------|---------|--------------|
| Onion skinning / Ghosting      | Yes  | Yes           | Yes     | No           |
| Motion trails (editable arc)   | Yes  | -             | Yes     | No           |
| Playblast / viewport capture   | Yes  | Yes           | Yes     | Screenshot only |
| Multiple viewports             | Yes  | Yes           | Yes     | No (single)  |
| Wireframe overlay              | Yes  | Yes           | Yes     | Debug modes   |
| Background image/reference     | Yes  | -             | Yes     | No           |
| Bone selection in viewport     | -    | Yes           | Yes     | Hierarchy only |
| Transform gizmo (translate/rotate/scale) | Yes | Yes  | Yes     | No           |
| Local/World space toggle       | Yes  | Yes           | Yes     | No           |
| Shading mode quick switch      | Yes  | Yes           | Yes     | Debug panel only |

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P0       | Onion skinning               | Show previous/next frames as semi-transparent overlays                         |
| P0       | Transform gizmo              | Interactive translate/rotate/scale handles in viewport                         |
| P1       | Motion trails                | Display trajectory arc of selected bone over time range                        |
| P1       | Viewport bone selection      | Click bones directly in 3D viewport to select                                  |
| P1       | Playblast                    | Record viewport as video sequence (image sequence or MP4)                      |
| P2       | Local/World space toggle     | Switch gizmo orientation between local and world space                         |
| P2       | Background reference image   | Load reference image behind scene for pose matching                            |
| P2       | Multiple camera presets       | Save/restore named camera views (Front/Side/Top/Perspective)                  |
| P3       | Split viewport               | Side-by-side viewports with independent cameras                                |

---

## 5. Workflow & Productivity

### Current State

- Undo/Redo (clip and schedule edits, max 128)
- Copy/Paste keyframes with mirror paste
- Bone naming pattern detection for mirror
- Snap to frame/keyframe
- Search filter in hierarchy
- Clip browser (create/load/save/duplicate/delete)

### Industry Standard

| Feature                        | Maya | MotionBuilder | Blender | This Project |
|--------------------------------|------|---------------|---------|--------------|
| Pose library                   | Yes  | Yes           | Yes     | No           |
| Animation retargeting          | -    | Yes (HIK)     | -       | No           |
| Editable motion paths          | Yes  | -             | Yes     | No           |
| Character sets / selection sets | Yes | Yes           | Yes     | No           |
| Hotkey customization           | Yes  | Yes           | Yes     | No           |
| Macro / script recording       | Yes  | Yes (Python)  | Yes     | No           |
| Animation snapshot / versioning | -   | Yes (Take)    | -       | No           |
| Preferences save/load          | Yes  | Yes           | Yes     | No           |
| Auto-save                      | Yes  | Yes           | Yes     | No           |
| Recent files list              | Yes  | Yes           | Yes     | No           |

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P0       | Pose library                 | Save/load named poses for quick posing (stored as clip at single frame)        |
| P1       | Selection sets               | Save groups of bones as named sets for quick selection                          |
| P1       | Hotkey customization         | User-configurable keyboard shortcuts                                           |
| P1       | Auto-save                    | Periodic auto-save of scene and clips                                          |
| P1       | Recent files list            | Remember recently opened model/animation files                                 |
| P2       | Animation retargeting        | Remap animation from one skeleton to another                                   |
| P2       | Take / version system        | Save multiple animation versions for comparison (like MotionBuilder Takes)     |
| P3       | Script / expression system   | Simple expression language for procedural animation                            |

---

## 6. UI / UX

### Current State

- ImGui-based UI
- Fixed grid layout (non-resizable panels)
- Dockspace-based main layout
- Floating curve editor window
- File dialogs via rfd crate

### Industry Standard

| Feature                        | Maya | MotionBuilder | Blender | This Project |
|--------------------------------|------|---------------|---------|--------------|
| Dockable/tabbed panels         | Yes  | Yes           | Yes     | Partial      |
| Custom layout save/restore     | Yes  | Yes           | Yes     | No           |
| Resizable panels               | Yes  | Yes           | Yes     | No (fixed)   |
| Dark/Light theme               | Yes  | Yes           | Yes     | Dark only    |
| Toolbar with tool icons        | Yes  | Yes           | Yes     | No (text buttons) |
| Status bar (FPS, frame, etc.)  | Yes  | Yes           | Yes     | No           |
| Tooltip help                   | Yes  | Yes           | Yes     | Minimal      |
| Right-click context menus      | Yes  | Yes           | Yes     | Partial      |
| Drag-and-drop between panels   | Yes  | Yes           | Yes     | Clip D&D only |
| Progress bar for operations    | Yes  | Yes           | Yes     | No           |

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P0       | Resizable panels             | Allow user to drag panel borders to resize                                     |
| P0       | Status bar                   | Show FPS, current frame/time, playback status, memory usage                    |
| P1       | Toolbar icons                | Visual toolbar with icons for common operations                                |
| P1       | Full right-click menus       | Context-sensitive menus for all interactive areas                               |
| P1       | Tooltips                     | Hover help text for all controls and buttons                                   |
| P2       | Layout save/restore          | Save window arrangement as named presets                                       |
| P2       | Progress indicators          | Show progress for model loading, baking, export                                |
| P3       | Theme customization          | User-selectable color themes                                                   |

---

## 7. Rendering Quality

### Current State

- Deferred rendering with G-Buffer
- Ray Query shadows
- Bloom (5-level mip chain)
- Depth of Field
- Auto Exposure (histogram-based)
- Tone mapping, Gamma, Vignette, Chromatic Aberration
- PBR materials (Base Color, Metallic, Roughness)

### Industry Standard (Viewport Renderers)

| Feature                        | Maya VP2 | Blender EEVEE | Unreal Viewport | This Project |
|--------------------------------|----------|---------------|-----------------|--------------|
| Screen-Space Reflections (SSR) | Yes      | Yes           | Yes             | No           |
| Screen-Space AO (SSAO/GTAO)   | Yes      | Yes           | Yes             | No           |
| Subsurface Scattering          | -        | Yes           | Yes             | No           |
| Transparent rendering          | Yes      | Yes           | Yes             | No           |
| Environment map / HDR sky      | Yes      | Yes           | Yes             | No           |
| Multiple lights                | Yes      | Yes           | Yes             | Single light |
| Soft shadows                   | Yes      | Yes           | Yes             | Hard only    |
| Anti-aliasing (TAA)            | Yes      | Yes           | Yes             | MSAA         |

### Improvements

| Priority | Item                         | Description                                                                    |
|----------|------------------------------|--------------------------------------------------------------------------------|
| P1       | SSAO                         | Screen-space ambient occlusion for depth perception                            |
| P1       | Environment map / HDR sky    | IBL lighting with HDR environment maps                                         |
| P1       | Multiple lights              | Support more than one light source                                             |
| P2       | Soft shadows                 | PCF or ray-traced soft shadows                                                 |
| P2       | Transparent rendering        | Alpha blending for transparent materials                                       |
| P2       | TAA                          | Temporal anti-aliasing for smoother edges                                      |
| P3       | SSR                          | Screen-space reflections for metallic surfaces                                 |
| P3       | Subsurface scattering        | For skin rendering quality                                                     |

---

## 8. Priority Summary

### P0 (Must Have)

1. **FBX animation export** - Core deliverable for production pipeline integration
2. **GLB animation export** - Web/game engine interop
3. **Onion skinning** - Essential animation workflow tool
4. **Transform gizmo** - Direct manipulation in viewport
5. **Keyframe range select** - Basic editing efficiency
6. **Multi-keyframe drag** - Dope sheet usability
7. **Weighted tangents** - Curve editor expressiveness
8. **More tangent types** - Industry-standard curve control
9. **Resizable panels** - Basic UI usability
10. **Status bar** - Frame/time/FPS display
11. **Pose library** - Animation workflow efficiency

### P1 (Should Have)

1. Pre/Post infinity modes (Cycle, Oscillate)
2. Box select in graph editor
3. Time display mode toggle (frames/seconds/timecode)
4. Playback range (in/out)
5. Scale keyframes in time
6. Arrow key keyframe nudge
7. Motion trails
8. Viewport bone selection (click in 3D)
9. Playblast (video export)
10. Selection sets
11. Hotkey customization
12. Auto-save
13. Animation-only export (no mesh)
14. Bake constraints before export
15. SSAO, Environment map, Multiple lights
16. Toolbar icons, Full context menus, Tooltips

### P2 (Nice to Have)

1. Buffer curves
2. Normalize curve display
3. Timeline markers
4. Reverse keyframes
5. Local/World space toggle
6. Reference image overlay
7. Camera presets
8. Layout save/restore
9. Animation retargeting
10. Take/version system
11. Alembic export
12. USD export
13. Soft shadows, Transparent rendering, TAA

### P3 (Future)

1. F-Curve modifiers
2. Audio track sync
3. NLA enhancements
4. Split viewports
5. Script/expression system
6. Theme customization
7. SSR, Subsurface scattering

---

## Unique Strengths (vs. Industry Tools)

This project has several features that are rare or absent in traditional DCC tools:

| Feature            | Description                                                             |
|--------------------|-------------------------------------------------------------------------|
| Curve Copilot (ML) | AI-powered keyframe suggestion based on context - not available in any major DCC tool |
| Text-to-Motion     | Generate animation clips from text prompts                              |
| ECS Architecture   | Data-oriented design enabling clean extensibility                       |
| Vulkan Ray Tracing | Modern GPU-accelerated rendering with ray query                        |
| Rust Safety        | Memory-safe engine without GC overhead                                  |

These differentiators should be preserved and enhanced as the project evolves.
