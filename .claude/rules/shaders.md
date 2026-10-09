---
paths:
  - "shaders/**"
  - "assets/shaders/**"
  - "crates/thyllore-vulkan-core/build.rs"
  - "crates/thyllore-shader-manifest/**"
  - "crates/thyllore-spirv-reflect/**"
---

# Shader System

## Compiling Shaders

Shaders are written in Slang and compiled by `crates/thyllore-vulkan-core/build.rs` during `cargo build`: every
`.slang` entry in `shaders/` is compiled to `assets/shaders/` with `slangc` (`SLANG_ROOT/bin/slangc`, default
`~/.local/slang`, installed by `scripts/setup_slang.sh`) and reflected from the SPIR-V. Stale `.spv` files with no
matching source are removed automatically. The same sources feed the CPU (`shaders/cpu/*_exports.slang`, compiled with
`-target cpp` and linked into `thyllore-effect-core` by its `build.rs`) and the Blender addons (`slangc -target glsl`
through `blender_addon/common/slang_export.py`), so the shader math has a single source.

The slangc release is pinned once, in `[package.metadata.slang]` of `crates/thyllore-shader-manifest/Cargo.toml`.
`scripts/setup_slang.sh` installs that release and every build script compares `slangc -v` against it and stops the
build on any other version, so the GPU SPIR-V and the CPU C++ always come from the same compiler.

## Shader Source Files

An entry file is any `.slang` file that declares at least one `[shader("<stage>")]` entry point; a module without one
(`include/`, `cpu/`) compiles to no SPIR-V. `build.rs` walks the tree, compiles every entry point of every entry file
with `-entry <name> -stage <stage>`, and mirrors each source directory under `assets/shaders/`, so file names only
have to be unique inside their directory. The `.spv` name is the stem, minus a trailing stage word if it has one,
plus the stage suffix:

- `model/raster.slang` (`vertexMain` + `fragmentMain`) -> `assets/shaders/model/rasterVert.spv` + `rasterFrag.spv`
- `editor/bone.slang` -> `assets/shaders/editor/boneVert.spv` + `boneFrag.spv`
- `wind/resolveFragment.slang` (`main`) -> `assets/shaders/wind/resolveFrag.spv`
- `raytracing/sceneClosestHit.slang` -> `assets/shaders/raytracing/sceneRchit.spv`
- stage words / suffixes: `Vertex`/`Vert`, `Fragment`/`Frag`, `Geometry`/`Geom`, `Compute`/`Comp`, `RayGen`/`Rgen`,
  `Intersection`/`Rint`, `AnyHit`/`Rahit`, `ClosestHit`/`Rchit`, `Miss`/`Rmiss` (`crates/thyllore-shader-manifest/src/stage.rs`
  is the one table)

A vertex + fragment pair that only one pass uses lives in one file with `vertexMain` and `fragmentMain`, so the
varyings struct, bindings and push constants are declared once (`model/raster.slang`, `editor/bone.slang`,
`postprocess/composite.slang`). A stage shared by several passes stays its own single-entry file with `main`
(`postprocess/tonemapVertex.slang`, `gbuffer/vertex.slang`).

Feature-specific modules live in a subdirectory with their own `include/` (`shaders/water/include/`), shared modules in
`shaders/include/`: `math.slang` (`PI` / `TWO_PI` / `HALF_PI`, `floorMod`; never re-declare them), `noise.slang`
(hash, gradient and value noise, `fbm3`, `hermiteFade`, `quinticFade`, IGN), `resample.slang` (tent kernels, 13-tap
downsample), `outline.slang` (id-image edge of an `IPixelClass`), `radiative_transfer.slang`. Every
non-entry file declares `module <name>;` with a feature prefix (`module water_lb;`) and is imported by its path from
the `shaders/` root (`import "water/include/lb.slang";`). slangc resolves imports relative to the entry's directory
first, so a feature `include/` must not reuse a shared include's file name. `passes.toml` references entry files by
their path relative to `shaders/` (`water/resolveFragment.slang`); a file contributes every entry point it declares.

Matrices: `mul(M, v)` is the GLSL `M * v`; chained products stay left-associative (`mul(mul(proj, view), p)`).
`M[3]` is a row, so read a translation column with `mul(M, float4(0, 0, 0, 1))`. Effect uniform blocks that several
passes bind at different slots are a `public static` in the effect's `component.slang`, filled once by each entry
(`water = waterBlock;`).

## Pass Manifest (`shaders/passes.toml`)

`shaders/passes.toml` is the only hand-written pass definition. Each `[pass.<name>]` lists its `stages`
(entry file paths relative to `shaders/`; the stages are the files' `[shader("..")]` entry points) and `sets` (set index -> role:
`frame` = 0, `material` = 1, `object` = 2, `local` = pass-owned). `crates/thyllore-vulkan-core/build.rs` validates the file
(missing source, orphan shader not referenced by any pass, bad stage composition, role/set convention) and
generates `PassId`, `PassShaders` constants and `ALL_PASSES` into `$OUT_DIR/pass_manifest.rs`.

- Create pipelines with `PipelineBuilder::from_pass(&FLAME_RESOLVE)` / `RRPipeline::new_compute_with_push_constants(.., &RAY_QUERY_SHADOW, ..)`.
- Declare layouts with `ReflectedLayoutSpec::shared(SetRole::Frame)` or `ReflectedLayoutSpec::local(&TONEMAP)`.
- `ReflectedLayoutSpec::for_role(pass, role)` derives the spec of any pass set from the manifest alone; the
  test `block_definition.rs` walks `ALL_PASSES` with it, so no Rust-side pass list exists.
- Adding a shader = Slang entry + `passes.toml` entry + its `*DescriptorSet`.

## Generated Binding Constants (`shader_bindings`)

`thyllore-vulkan-core/build.rs` also generates `$OUT_DIR/shader_bindings.rs` from the SPIR-V reflection: one
`pub mod <pass_name>` per pass with one `pub const <NAME>: ShaderBinding { set, binding, kind, count }` per
descriptor, named from the Slang identifier (`historySampler` -> `HISTORY_SAMPLER`). Two stages of one pass declaring
the same (set, binding) with different block layouts is a build error (names may differ). No hand-written `*_BINDING: u32` constants exist;
renaming a descriptor in Slang breaks the Rust build at the referencing site.

- Write descriptors through `layout.writer(set).buffer(flame_resolve::FLAME, ..)` — the writer takes `ShaderBinding`
  and rejects a kind that the layout slot does not accept.
- `ReflectedLayoutSpec::with_override(shader_bindings::.., vk::DescriptorType::..)` overrides a descriptor type.
- `PipelineBuilder::descriptor_layouts(&[&ReflectedSetLayout])` / `RRPipeline::new_compute*` verify at pipeline
  creation that every (set, binding) used by the pass shaders exists in the given layouts with a matching type.
- A pass whose stages declare a `push_constant` block also gets `pub const PUSH_CONSTANT: PushConstantLayout`
  (block name, declaring stages, size); every stage must declare the identical block, so no `layout(offset = ..)`
  and no hand-written offset or size constants exist. `push_constant_range(&<pass>::PUSH_CONSTANT)` builds the
  `vk::PushConstantRange` (`effect_trace` uses it).
- A `ConstantBuffer<WaterUBO> waterBlock` is reflected under its instance name (`WATER_BLOCK`); a
  `StructuredBuffer<T>` reflects as a block whose layout, not name, must match across stages.

## Generated GPU Block Structs and C ABI Bindings

The Rust struct that mirrors a Slang uniform or push constant block is never written by hand, and neither is the
`extern "C"` binding of a CPU export. Both are generated into `OUT_DIR` on every build, so there is no checked-in
generated file, no regeneration command and no golden test:

- `FlameUBO`, `WaterUBO`, `WindUBO`: `crates/thyllore-effect-core/build.rs` compiles `shaders/cpu/<effect>_exports.slang`
  with `-target cpp`, parses the C++ it gets back (`thyllore-shader-manifest` `cpp_abi`), and writes
  `<effect>_gpu_blocks.rs` (a `declare_gpu_block!` per struct, included by `<effect>/gpu/components/mod.rs`) and
  `<effect>_exports_bindings.rs` (the `extern "C"` block, `KernelContext` / `GlobalParams` and `FloatN`, included by
  `<effect>/analytic/slang.rs`). The struct therefore has the C++ natural layout the exports read.
- The same build also compiles the effect's resolve stage to SPIR-V, reflects the block's std140 layout, refuses to
  build when any member offset or size differs from the natural layout, and appends `static_assert`s on
  `sizeof` / `offsetof` of the C++ structs to the generated C++ so the C++ compiler proves the equality too.
- Blocks with no CPU export are generated from the SPIR-V that declares them through the shared
  `thyllore-shader-manifest` helpers (`reflect_stage_block`, `generate_spirv_block_rust`): `TracePush` by
  `crates/thyllore-vulkan-core/build.rs` (`GPU_BLOCKS` there) into `trace_push.rs`, and `LightningUBO` /
  `LightningSegmentsUBO` (lightning builds its segments in Rust, so no Slang CPU module proves a natural layout) by
  the effect-core `build.rs` (`GPU_ONLY_BLOCKS`) into `lightning_gpu_blocks.rs` / `lightning_segments_gpu_blocks.rs`.
- `volume_exports.slang` has no global block; its exports take `VolumeShell` by pointer, and the Rust struct that the
  binding names must be `#[repr(C)]` and in scope at the `include!` site.

After changing a block or an export in Slang, `cargo build` regenerates everything; fix the Rust call sites the
compiler points at. Adding a CPU module = one `shaders/cpu/<name>_exports.slang` with `export __extern_cpp` wrappers, one
`CpuModule` entry in the effect-core `build.rs`, and one `include!` site. Every global the module reads
(`ConstantBuffer<T>`, samplers) lands in `GlobalParams`; the Rust mirror is built with `GlobalParams::new(&ubo)`.

## Shader Modifications

After editing shaders in `shaders/`, the build system automatically compiles them to `assets/shaders/` directory during
`cargo build`. The application loads compiled shaders from `assets/shaders/` directory.
