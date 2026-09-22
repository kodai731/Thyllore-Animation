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
matching source are removed automatically. The same sources feed the CPU (`shaders/cpu/exports.slang`, linked into
`thyllore-effect-core` by its `build.rs`) and the Blender addons (`slangc -target glsl` through
`blender_addon/common/slang_export.py`), so the shader math has a single source.

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

## Generated GPU Block Structs (`generate_gpu_blocks`)

The Rust struct that mirrors a Slang uniform or push constant block is never written by hand. It is generated from
the compiled SPIR-V into a checked-in file (`FlameUBO`, `WaterUBO`, `WindUBO` under
`crates/thyllore-effect-core/src/<effect>/gpu/components/generated.rs`, `TracePush` under
`crates/thyllore-vulkan-core/src/renderer/trace_push.rs`; the list is `GPU_BLOCK_TARGETS` in
`thyllore-shader-manifest`).

After changing such a block in Slang:

```bash
cargo build                                                    # compiles the SPIR-V the generator reads
cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks
```

Then fix the Rust call sites that construct the struct. The `block_definition` golden test fails with this command
in its message whenever a checked-in file is stale, so forgetting the step cannot pass the tests.

The block struct is declared once in the effect's `include/component.slang` and bound as `ConstantBuffer<T>` or read
through a `T*` device address; the generator finds the struct through the wrapping block, so the Rust struct keeps
the struct's name. Slang wraps `T[N]` members in `_Array_std140_*` structs, which the reflection unwraps. Adding a block = a `GpuBlockTarget` entry, a `pub mod` for the output file, and
running the command once.

## Shader Modifications

After editing shaders in `shaders/`, the build system automatically compiles them to `assets/shaders/` directory during
`cargo build`. The application loads compiled shaders from `assets/shaders/` directory.
