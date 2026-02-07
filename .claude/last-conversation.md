# Context Memory

## Project / Topic
- Rust Vulkan rendering engine, ECS architecture, branch feature/rig-system
- Phase 9: Advanced Visualization Features implementation

## Goals
- In Front Toggle: bone depth test ON/OFF
- Two-Pass Rendering: visible solid, occluded semi-transparent (alpha 0.25)
- Custom Bone Shapes: Box and Sphere presets
- Distance-Based Scaling: camera distance-responsive bone sizing

## Key Decisions
- Push constant `float alpha` in boneFragment.frag for transparency control
- 4 new pipelines: bone_solid_depth (LESS), bone_wire_depth (LESS), bone_solid_occluded (GREATER+blend), bone_wire_occluded (GREATER+blend), all depth write=false
- Box mesh: 8 verts, 12 tris, width = bone_length * 0.08
- Sphere mesh: UV sphere, 6 rings x 8 segments, radius = bone_length * 0.06
- Visual scale: (camera_distance * factor).max(0.1)
- UI: radio buttons (Stick/Octa/Box/Sphere), In Front checkbox, Distance Scaling checkbox + Factor slider

## Files Modified
- shaders/boneFragment.frag: push constant alpha
- src/debugview/gizmo/bone.rs: BoneDisplayStyle Box/Sphere, new fields, 4 RenderInfo
- src/app/init/instance.rs: 4 new pipelines, push_constants on existing
- src/renderer/deferred/composite.rs: two-pass draw_bone_gizmo
- src/ecs/systems/bone_gizmo_systems.rs: Box/Sphere mesh generation
- src/ecs/systems/phases/render_prep_phase.rs: Box/Sphere update, compute_visual_scale
- src/ecs/systems/render_data_systems.rs: Box/Sphere render data match
- src/platform/ui/hierarchy_window.rs: bone display settings panel
- src/ecs/events/ui_events.rs: 4 new UIEvent variants
- src/ecs/systems/ui_event_systems.rs: new events in catch-all
- src/platform/events.rs: event handlers for bone display settings

## Constraints & Rules
- Build: OK (0 errors), Unit tests: 59 passed, Integration tests: 30 passed + 1 pre-existing failure
- ECS architecture checker confirmed compliance

## Open Questions
- Runtime verification pending: switch Stick/Octa/Box/Sphere, toggle In Front, test Distance Scaling
