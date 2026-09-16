---
paths:
  - "src/**"
  - "crates/**"
---

# Directory Hierarchy and File Responsibilities

Update this file whenever the directory layout changes. Before creating a file, decide its home with the
checklist at the end and never leave it in `src/app/` "for now".

## Dependency direction

```
crates/*-core            pure domain + GPU primitives, no ECS (vulkan-core knows Vulkan, nothing knows World)
    ▲
src/ecs/                 World, components, resources, systems, phases (all engine logic lives here)
    ▲
src/app/ , src/platform/ App lifecycle, Vulkan object ownership, frame driver, window / imgui / input
```

Lower layers never import upper ones: a crate never depends on `src/`, and `src/ecs/` never imports
`src/app/`. A system that needs GPU objects receives a context struct that borrows exactly what it uses
(`FrameContext`: device, instance, command pool, `World`, assets, graphics resources, raytracing data, frame
slot), never `App` or `AppData`. If a system seems to need `App`, it is either app wiring (move it to
`src/app/`) or the context is missing a field (add the field).

### Known exceptions (tracked in #163, do not add to this list)

The following places still import `crate::app` from `src/ecs/`. They are violations kept only until #163
lands; new code must not copy them, and no new entry may be added here.

- `FrameContext` and `LightMoveTarget` are defined under `src/app/` although they contain no `App`; the
  fix is to move them into `src/ecs/`.
- `src/hooks/effect.rs` declares `on_viewport_resize` as `fn(&mut App)` and `src/hooks/pass.rs`
  declares `RenderPassNode::prepare` as `fn(&mut App)` and `record` / declarations as `fn(&App)`, so
  `src/ecs/systems/{flame,water}/render_targets.rs` take `&mut App`; the fix is an `EffectContext` that
  borrows instance, device, viewport pools and extent, raytracing data and `World` (the `setup` hook
  already takes decomposed arguments).
- `src/ecs/systems/phases/render_phase.rs::build_frame_render_context` takes `&App` to read four fields.

## crates/

A crate holds code that is meaningful without the engine: pure math, domain types and their pure
operations, GPU primitives, importers and exporters, codegen used by build scripts.

- Every crate is named `thyllore-<topic>-core` (or `-api`, `-debug`, `-client`, and `-derive` for the
  proc-macro companion of a `-core` crate) and states its layer and what it must not depend on in
  `Cargo.toml` `description`.
- A crate never depends on `World`, `Entity`, `AssetStorage` or `App`.
- Only `thyllore-vulkan-core` (and the debug crate) may name `vk::*`. Everything else describes GPU work
  through the abstract types of `thyllore-render-core`.
- Effects (`thyllore-effect-core`) keep one directory per effect with the same split: `effect/` data,
  `analytic/` pure math mirrored by the shaders, `gpu/` UBO structs, `presets`, `settings`.
  Analytic pieces that more than one effect can use (`volume/`: ray knots, the wall shell, puffs) live
  beside the effect directories and mirror `shaders/include/`; an effect's `analytic/` only builds the
  shared structs from its own parameters and adds what is specific to it (wind: streak / eddy modulation).
- `thyllore-vulkan-core` never names an effect: no `Wind*` / `Flame*` / `Water*` types, no per-effect
  fields in `RayTracingData`, no per-effect descriptor set, record helper, buffer or push constants. It
  offers generic primitives only (images and `VolumeImage`, samplers, `create_color_overlay_render_pass`,
  `ReflectedSetLayout`, `UniformBuffer<T>`, `PipelineBuilder`); an effect's GPU state is an ECS resource
  (`src/ecs/resource/<effect>_render_targets.rs`) assembled from those primitives in
  `src/ecs/systems/<effect>/` (descriptors, pipeline, record, render targets). Wind follows this layout.
  Known exceptions tracked by the vulkan-core effect-neutral issue (#179), not to be extended:
  `descriptor/flame.rs`, `descriptor/water.rs`, `descriptor/water_caustic.rs`, `descriptor/effect_trace.rs`,
  `renderer/flame.rs`, `renderer/water.rs`, `resource/flame_buffer.rs`, `resource/water_buffer.rs`, the
  flame / water fields of `RayTracingData` (filled by `src/ecs/systems/<effect>/pipeline.rs`), and
  `FlamePushConstants` / `WaterPushConstants` in `renderer/push_constants.rs`. Pipeline creation never
  lives in vulkan-core: effects build theirs in `src/ecs/systems/<effect>/pipeline.rs`, the core
  post-process passes in `src/app/post_process/pipelines.rs`, onion skin in `src/app/init/onion_skin.rs`.
- Domain crates use the `components/` (data) and `systems/` (pure functions) split, see
  `ecs-architecture.md`.
- GPU object lifetime: a type that owns Vulkan handles implements `GpuResource` (`resource/gpu_resource.rs`)
  next to its own `destroy`; a type that only aggregates such fields writes `#[derive(GpuResource)]`
  (`thyllore-vulkan-derive`) and never enumerates them, `#[gpu_resource(skip)]` marks a borrowed handle.
  Destroy order is the reverse of field declaration order, and `resource_name` is the type name, never a
  hand-written string. `AppData` is itself derived, so a new GPU field on it fails to compile until its type
  implements `GpuResource`. A `World` resource that owns GPU objects derives the trait and writes
  `gpu_resource!(Type)` next to its definition (`src/hooks/gpu_resource.rs`, link-time `inventory`);
  `App::destroy` runs every registration, then `AppData`, then the swapchain-level resources in fixed
  order, then the device, surface, messenger and instance, so the validation layer's leak check at
  `vkDestroyDevice` is the final proof. A test in `src/hooks/gpu_resource.rs` fails when a file in
  `src/ecs/resource/` declares a GPU-owning field without `gpu_resource!`.

Rule of thumb: if the code needs neither `World` nor `vk::*`, it belongs in a crate. If it needs `vk::*`
but not `World`, it belongs in `thyllore-vulkan-core`.

### Crates as Blender addon build units

Some crates are also shipped standalone as Python wheels for the Blender addons, built with maturin
(`crate-type = ["cdylib", "rlib"]`, `python` feature, abi3-py310):

- `thyllore-ml-core` → the curve copilot addon (`blender_addon/`, wheels collected by
  `scripts/collect_wheels.{sh,ps1}`, packaged by `scripts/build_blender_addon.{sh,ps1}`).
  `thyllore-ml-api` is the ABI marker shared between that wheel and the addon; changing it requires a
  coordinated wheel rebuild and end-user reinstall.
- `thyllore-effect-core` → the flame and water addons (`blender_addon/effects/<effect>/`, release workflows
  `blender_<effect>_addon_release.yml`). One crate serves every effect; the addon imports the analytic
  core and presets from it so the Blender preview and the engine share the same math.

Consequences for placement:

- Anything the addon needs (analytic math, presets, settings, scene format, UBO layouts, `pybindings/`)
  must live in one of these crates or their pure dependencies, never in `src/` or `thyllore-vulkan-core`.
- These crates must stay free of ECS, Vulkan and rendering-pipeline dependencies, or the wheel stops
  building.
- The `python` feature only adds the PyO3 facade; the same code compiles as an `rlib` for the engine, so
  the Rust API is the single source of truth for both.
- The Blender addon is the only consumer of `crate-type = "cdylib"`; do not add it to other crates.
- `thyllore-grpc-client` is exercised against Blender in CI (`blender_grpc_parity.yml`) but is not a
  wheel; it stays a plain library crate.

## src/ecs/

Everything that decides what the engine does: components and resources (data only), systems (logic, one
file per domain, one directory per effect), phases (execution order and event dispatch), and the ECS core
(world, storage, query, registry, events). Rules are in `ecs-architecture.md`.

No file here declares `impl App` or takes `&mut App` (see the known exceptions above). A system that needs
GPU resources as well as `World` takes `FrameContext` or a smaller context struct (`raytracing_systems.rs`:
per-frame TLAS refresh from `GlobalTransform`). If the work is mostly GPU upload and rebuild with a few
`World` writes, it is app wiring and lives in `src/app/` (`model_loader/`, `scene_model.rs`); if it exists
only for debugging (debug primitive spawn / delete) it lives in `src/debugview/`.

## src/hooks/

Generic hook infrastructure that lets a subsystem plug into the app lifecycle without being named by
`src/app/`. `effect.rs` holds the effect hook (setup, viewport resize, pass nodes) and the list that runs
them in subscription order; GPU teardown is not a hook, it is the `gpu_resource!` registration of the
effect's resource (`gpu_resource.rs`: `GpuResourceHook`, `GpuResourceHooks::collect()`). `pass.rs` holds the `RenderPassNode` contract (name,
stage, `transients` requested by slot and desc, reads / writes declared as `TargetUse`, `prepare`, record),
the `PassStage` order (lighting → effect → post-process → final) and the `PassGraph` that keeps registered
nodes sorted by stage then registration order. The graph runner in `src/app/command_recording.rs` runs
three phases per frame: build (collect requests and uses, compute `TransientLifetimes`, acquire each slot
at its first use and release it after its last, `src/app/pass_targets.rs`), prepare (nodes bind the frame's
images into descriptors and framebuffers), record (`ImageStateTracker` in `thyllore-vulkan-core`
`renderer/pass_target.rs` emits the layout barriers). Pass code never acquires a transient or writes an
`ImageMemoryBarrier` for a declared target; `AppData.frame_transients` is the single map from slot to
handle. `scene.rs` holds the `SceneComponentHook` contract (type key, owner / attachment role,
entities, capture, apply), the `scene_owner!` / `scene_attachment!` macros that submit a hook to the
link-time registry (`inventory`), and `SceneComponentHooks::collect()` that `src/app/` stores as a
`World` resource for `src/scene/`; owners are applied before attachments. `scene_resource.rs` is the
same contract for world resources (`SceneResourceHook`, `scene_resource!`, `SceneResourceHooks`).
`gpu_primitive.rs` holds the `GpuPrimitiveSource` contract (a component that describes its ray-tracing
instance as a `GpuPrimitive`, plus `effect_data_address(world, ordinal)` for the device address of the
instance block its closest hit shader reads through the hit record), the `gpu_primitive_source!`
registration and `collect_all(world)`, which the acceleration structure build and the per-frame TLAS
refresh both call so the instance order is one list (hooks sorted by type name, entities sorted by id).
The effect trace pass (`shaders/raytracing/`, `raytracing_systems.rs::ensure_effect_trace_pipeline`) is
shared by every effect: its descriptor set is effect independent and an effect's own data reaches its hit
shader only through that address, never through an extra descriptor set or push constant. A hook file
describes a contract only; it never names a concrete effect.

## src/effect/

The one place that subscribes the effects (`subscription.rs`): it lists the hook constants of flame, water
and any future effect for the `EffectHook` contract (GPU lifecycle and pass nodes). Scene components
are not listed here: they register at link time from their own files (`scene_owner!` /
`scene_attachment!`). `src/app/` runs the hooks generically and never names an effect; an effect's own
systems (`src/ecs/systems/<effect>/`) implement the hook and own the effect's GPU state as an ECS
resource. Adding an effect means adding its `EffectHook` constant to `subscription.rs`, nothing in
`src/app/` or `src/scene/`. Subscription order is also the record order of the effects' pass nodes
inside the effect stage.

## Feature isolation: no effect names outside the effect's own directories

Every feature (today the effects flame, water, wind) is a set of directories that only it may name. Code
outside those directories reaches a feature through a contract (`src/hooks/`), a registry it subscribes to
(`src/effect/subscription.rs`), or reflection metadata the feature's own declaration generates
(`declare_scene_format!` → `SceneComponent::TYPE_KEY` / `PERSISTED_FIELDS`, `ScalarChannelDomain`,
`UiParam` tables). Directory position decides what a file may see:

| Directory | May name flame / water / wind |
|---|---|
| `crates/thyllore-effect-core/src/<effect>/`, `shaders/<effect>/` | its own effect only |
| `src/ecs/component/<effect>*.rs`, `src/ecs/systems/<effect>/`, `src/ecs/resource/<effect>_*.rs` | its own effect only |
| `src/effect/subscription.rs` | every effect (the single `EffectHook` list; scene hooks self-register instead) |
| `src/platform/ui/` per-effect windows, `src/debugview/` per-effect dumps | the effect the file is for |
| `src/scene/`, `src/hooks/`, `src/ecs/systems/*.rs` (shared systems), shared crates | none, tests included (`src/scene/` tests use `entities.rs::test_support`; effect round trips live in `src/ecs/systems/<effect>/tests.rs`) |
| `src/app/`, `src/ecs/world.rs` | none in new code; the existing spots (default flame spawn in `init/instance.rs`, `query_flames` / `query_waters` / `query_winds`) are exceptions tracked with #179 and must not grow |

Concretely:

- `src/scene/` persists entities as `(name, components{type_key → value})`. It never writes
  `FlameSceneData`, `"water_torus"`, `apply_wind_state_to_world` or an effect field name (`column_height`,
  `sigma_t`). What it may do: iterate the `SceneComponentHooks` resource (`src/hooks/scene.rs`) that
  `src/effect/subscription.rs::subscribe_scene_components` filled, and decode through the
  `thyllore_scene_core::SceneComponent` trait. The hooks are generic:
  `SceneComponentHook::owner::<C>()` for a component that defines its entity (`C: SceneOwner`, the
  engine-side trait giving icon and placement, implemented in `src/ecs/component/<effect>.rs`) and
  `SceneComponentHook::attachment::<C>()` for anything restored by insertion; the type key comes from
  `<Effect>::TYPE_KEY`, which `declare_scene_format!` generated from the `key:` item. Hooks are
  registered at link time (`inventory`): `scene_owner!(Effect { icon, placement, prepare_loaded? })`
  and `scene_attachment!(C)` in the component's own file both submit the hook, and
  `SceneComponentHooks::collect()` gathers every submission at app start (duplicate keys fail there).
  There is no list of scene components anywhere.
- Adding a persisted parameter = one entry in the effect's `declare_scene_format!` table. Nothing in
  `src/scene/` changes. Adding an effect = `scene_owner!` in its component file; a provenance component
  = `scene_attachment!`. Runtime-only companions (baked data, accumulators) are inserted by the effect's
  own per-frame system when missing, never by the loader.
- `src/hooks/` files describe contracts (`EffectHook`, `RenderPassNode`, `SceneComponentHook`); they take
  fn pointers and `&'static str` keys, never an effect type.
- `src/ecs/world.rs` offers generic component access (`iter_components::<C>`, `insert_component`); it does
  not grow `with_<effect>()` builders or `query_<effect>s()` helpers. The existing `query_flames` /
  `query_waters` / `query_winds` are tracked as exceptions and must not be extended.
- Crates depend downward only: `thyllore-effect-core` depends on `thyllore-scene-core` / `-math-core` /
  `-color-core`, never on a sibling feature crate or on `src/`. Two features never depend on each other's
  crate or module; anything two features share moves down into the shared parent
  (`crates/thyllore-effect-core/src/volume/`, `thyllore-scene-core`, `src/hooks/`).

```rust
// Bad: src/scene/ names an effect and one of its fields
pub struct WindSceneData { pub effect: WindTornadoEffect, pub preset: Option<String> }
fn apply_wind(world: &mut World, wind: &WindSceneData) { effect.column_height = wind.effect.column_height; }

// Good: src/scene/ iterates the registry; the effect registered its own hook
for hook in world.resource::<SceneComponentHooks>().ordered() {
    if let Some(value) = scene_entity.components.get(hook.type_key) {
        (hook.apply)(world, assets, entity, value)?;
    }
}
```

Test for it before finishing: `grep -rni "flame\|water\|wind" src/scene src/hooks` must hit nothing but
the stub pass names of `src/hooks/pass.rs` tests.

Resources follow the same rule. A resource is persisted by declaring its fields once
(`declare_scene_format!` in the resource's own file, or in its crate for `thyllore-render-core` settings)
and writing `scene_resource!(Type)` next to it; `SceneResourceHooks::collect()` gathers every registration
at link time and `src/scene/` writes them under `resources`. A resource whose persisted form needs `World`
context (the timeline names its active clip) submits a hand-written `SceneResourceHook` from its system
file. Enum fields are persisted by name through a `String` field (`ToneMapOperator::name` /
`from_name`), because a `ron::Value` cannot carry a unit variant.

## src/platform/

Window, input, imgui orchestration and the UI windows. Reads resources, records `UIEvent`s, calls one
dispatch entry point. Contains no business logic and no Vulkan commands beyond imgui rendering.

## src/vulkanr/

App-side Vulkan glue: ECS resources that wrap swapchain, sync objects and core attachments, per-frame pass
recording that has to read `App`, the core pass nodes (`renderer/deferred/nodes.rs`: composite, onion skin,
bloom, dof, auto exposure, tonemap registered into the `PassGraph`), and implementations of app-side backend
traits. Anything here that turns
out to need only device and handles moves down into `thyllore-vulkan-core`.

## src/render/

The app-side extension of the abstract render backend: traits whose signatures must mention an ECS
resource, plus re-exports of `thyllore-render-core` handle types. It is not a renderer and must stay small.

## src/app/

The application shell. It owns the Vulkan instance and device, the `AppData` aggregate and the viewport,
and drives one frame. It is the only place that sees `App` as a whole.

Files: `init/` and `cleanup.rs` (construction, teardown), `data.rs` (`AppData`), `viewport.rs` (core
attachments, storage and transient pools), `render.rs` (frame driver), `update.rs` (per-frame update and
imgui buffers), `command_recording.rs`, `model_loader/` and `scene_model.rs` (model load entry points,
texture file resolution, `World` registration, initial pose; GPU mesh creation and vertex upload are
`GraphicsResources::push_mesh` / `upload_mesh_vertices` in `thyllore-vulkan-core`), `raytracing/`
(acceleration structure rebuild), `frame_context.rs` and `render_context.rs`, `post_process/`,
`features/` (see below), `util.rs`, `color_test_quad.rs`.

`src/app/*.rs` is the core loop only. Optional capabilities that extend `App` but are not needed to drive a
frame live in `src/app/features/<feature>.rs` (Unreal's modular features, bevy's optional plugins):
`screenshot.rs` (swapchain and image readback to a host buffer, PNG encoding). A feature may be removed
without touching the frame loop; if removing it would break `begin_frame` / `render`, it is not a feature.

Belongs here:
- `App` construction and teardown
- ownership containers whose lifetime is the app or the viewport (core attachments, the render target
  storage and transient pools)
- the frame driver: begin frame, update, record, submit, present, swapchain recreation
- optional capabilities under `features/` (screenshot); `src/debugview/` builds on
  `features/screenshot.rs::copy_image_to_buffer`
- context structs that bundle `App` fields for callees
- wiring that must touch several subsystems at once (a resize fan-out, rebinding after a resize)
- calls into the hook infrastructure of `src/hooks/` (setup, viewport resize, GPU resource teardown) and
  the pass graph without naming an effect
- core post-processing passes (tonemap, auto exposure, dof, bloom) as one concept under
  `src/app/post_process/`: pipeline creation, resize rebinding and the per-frame descriptor binding that
  the core pass nodes call from `prepare` live together there. These are engine passes, not effects, so
  their nodes are registered by `App` directly

Does not belong here:
- ECS domain logic (querying, mutating components, deciding what an effect does) → `src/ecs/systems/`
- Vulkan helpers that need only device and handles (barriers, render passes, copies) → `thyllore-vulkan-core`
- pure math or analytic code → `thyllore-math-core`, `thyllore-effect-core`
- per-effect pass recording, resize or descriptor updates → `src/ecs/systems/<effect>/`
- anything that exists only for debugging (dumps, debug primitives) → `src/debugview/`
- UI drawing → `src/platform/ui/`

A function that takes `&mut App` only to read a few fields is misplaced: pass those fields in and put it
where the checklist says.

Effects own their GPU state: the buffers of an effect are an ECS resource
(`src/ecs/resource/<effect>_render_targets.rs`, derived `GpuResource` + `gpu_resource!`), creation and
resize are systems in `src/ecs/systems/<effect>/render_targets.rs` exposed as an effect hook subscribed in
`src/effect/subscription.rs`, and per-frame images are requested by the effect's pass nodes in
`src/ecs/systems/<effect>/passes.rs` (the graph acquires them). `src/app/` never enumerates effects (this mirrors bevy's `TextureCache` + per-effect `prepare_*` systems and Unreal's RDG +
per-feature `AddPass`).

## Other src/ directories

- `src/scene/` — the scene file schema (`file.rs`: version, metadata, model path, clip files,
  `resources{type_key → value}`, `entities[(name, components{type_key → value})]`), load / save
  (`scene_io.rs`), the generic entity capture / apply (`entities.rs`) and clip io. It holds no mirror
  struct of any resource or component: every persisted type registers itself (`scene_resource!`,
  `scene_owner!`, `scene_attachment!`) from its own file, and the capture / apply of a hook that needs
  `World` logic is an ECS system (`src/ecs/systems/scheduled_clip_systems.rs`,
  `debug_primitive_systems.rs`, `timeline_systems.rs`), never a file under `src/scene/`. Tests in
  `src/scene/` use the test-only `ProbeOwner` / `ProbeLabel` of `entities.rs::test_support`; a test that
  needs a concrete effect belongs to that effect's `tests.rs`
- `src/asset/` — CPU-side model asset storage
- `src/debugview/` — `impl App` blocks that exist only for a debugging session: GPU image and buffer dumps
  (`flame_history_dump.rs`, `water_debug_dump.rs`, `exposure_dump.rs`, `shadow_debug.rs`) and debug scene
  manipulation (`debug_primitive.rs`: cube / sphere / floor spawn and entity delete), one file per subject.
  This is the only directory outside `src/app/` that may extend `App`; it reuses the readback helpers of
  `src/app/features/screenshot.rs`. CPU mirrors of shader math go to `thyllore-render-debug` instead
- `src/ml/` — inference thread, feedback, licensing worker
- `src/logger/` — logger and message buffer
- `src/loader/`, `src/exporter/`, `src/grpc/`, `src/math/`, `src/animation.rs` — thin `pub use` shims over
  the crates; add nothing else there
- `src/bin/` — standalone tools

## The three "render" places

- `crates/thyllore-render-core` — how systems talk about GPU work without naming Vulkan: the backend trait,
  handles, UBO layouts, post-process settings.
- `crates/thyllore-vulkan-core` (`backend.rs`, `renderer/`) — the Vulkan implementation of that trait and the
  per-pass command helpers that take an immutable frame render context.
- `src/render/` and `src/vulkanr/` — the app-side extension of the trait (needs ECS resource types) and its
  Vulkan implementation, plus pass recording that reads `App`.
- `src/app/render.rs` — the frame driver: begin and end of a frame, swapchain-level concerns, the resize
  fan-out, auto exposure readback and the gbuffer / billboard / imgui recording it does directly. It names no
  effect and no model format; per-frame TLAS refresh is `src/ecs/systems/raytracing_systems.rs`, model
  loading is `scene_model.rs`.

Frame contexts, innermost to outermost: the crate-level immutable render context (device, resources,
pipelines, image index) → the app render context (mutable GPU resources, builds the backend) → the app frame
context (adds `World`, assets, time, frame slot) used by the ECS phases.

## Render target ownership

- Lender (engine, in the viewport): Storage for images that must survive a frame (history, accumulation;
  viewport-extent lifetime, keyed by purpose, reset on resize) and Transient for images that live inside one
  frame (described by extent / format / usage, handed out as frame-stamped handles, recycled per
  frame-in-flight bucket).
- Borrower (the pass or effect): asks the lender each frame, keeps what it borrowed in its own state
  (post-process targets under `src/app/post_process/`, effect resources under `src/ecs/resource/`), and keeps
  one descriptor set per frame slot for anything transient.
- Core attachments (HDR, depth, gbuffer, offscreen) are owned by the viewport and never pooled.

## Where does a new file go?

1. Needs neither `World` nor `vk::*` → a crate.
2. Needs `vk::*` but not `World` or `App` → `thyllore-vulkan-core` (`resource/` for objects, `renderer/` for
   command helpers, `descriptor/` for sets).
3. Needs `World` (query, mutate, decide) → `src/ecs/systems/`, one file per domain or one directory per
   effect. Data types go to `src/ecs/component/` or `src/ecs/resource/`.
4. Needs `App` because it wires several subsystems or owns app-lifetime Vulkan objects → `src/app/`.
5. Needs `App` only to read a handful of fields → not `src/app/`; pass those fields in (or extend
   `FrameContext`) and apply rules 1–3. `impl App` is allowed only in `src/app/` and `src/debugview/`.
6. Draws imgui → `src/platform/ui/`.
7. Exists only for debugging (dumps, debug primitives) → `src/debugview/` (needs `App`) or
   `thyllore-render-debug` (CPU mirror).
8. Extends `App` with an optional capability the frame loop does not need (screenshot, export) →
   `src/app/features/`.
