---
paths:
  - "src/**"
  - "crates/**"
  - "scratch/**"
  - "Cargo.toml"
---

# Module Structure

**IMPORTANT:** Every public item of the workspace's own crates has exactly one public path, and that path
contains only names that are part of the API. Decide the shape below before adding a `mod`, a `pub use` or a
`pub` item; the rules apply to every `.rs` file under `src/` and `crates/`, not only to `mod.rs`.

The checks are `crates/thyllore-grpc-client/tests/module_facade/` (R1-R6, syntax only, no exception table) and
`[workspace.lints.rust]` in the root `Cargo.toml`. They are the source of truth for the exact decision; this
file states what to write so that they pass.

```bash
cargo test -p thyllore-grpc-client --no-default-features --test module_facade
```

## Facade or namespace

A module that other code can reach is one of two shapes. Never mix them for the same child.

| | Facade | Namespace module |
|---|---|---|
| Shape | `mod x;` or `pub(crate) mod x;`, + `pub use x::Item;` | `pub mod x;` and no re-export of `x` |
| Public path | `parent::Item` | `parent::x::Item` |
| The name `x` | where the code is stored; invisible outside | part of the API; names a concept |
| Use when | the split exists to keep files small | the module name is a stable concept |

Decide with one question: if this file were split in two, may the callers' paths break? If they must not
break, it is a facade. If the module name itself is the promise, it is a namespace. This is the difference
between `std::collections::HashMap` (defined in `hash::map`, which is not shown) and `std::fs::File` (`fs`
is the concept).

How the workspace applies it:

- First level under a crate or layer is a namespace: `thyllore_vulkan_core::resource::X`,
  `thyllore_effect_core::flame::X`, `crate::ecs::resource::X`, `crate::hooks::scene::X`.
- The level below is file layout and stays private: `graphics_resource`, `analytic`, `gizmo`,
  `components::chain_gen`. A domain's `mod.rs` is the facade that names what it exposes.

```rust
// Good: facade. Callers write `cloth_setup::ChainGenParams`; the file layout can change freely
pub(crate) mod components;
pub(crate) mod systems;

pub use components::chain_gen::ChainGenParams;
pub use systems::chain_gen::place_chains;

// Good: namespace. Callers write `hooks::scene::SceneComponentHook`
pub mod scene;
pub mod pass;

// Bad: both shapes at once, `ChainGenParams` has two public paths
pub mod components;
pub use components::chain_gen::ChainGenParams;
```

## What the checks require

### R1: `mod.rs` and `lib.rs` hold declarations and re-exports only

Allowed items: `mod x;`, `use`, `pub use`, `extern crate`, and `#[proc_macro*]` functions (Rust requires
those in the crate root). Everything else goes into a sibling file: `struct`, `enum`, `trait`, `type`,
`const`, `static`, `fn`, `impl` blocks, macro invocations and inline `mod x { .. }`.

### R2: a public module is not re-exported from the same file

`pub mod x;` together with any `pub use x::..` in one file is a dual path. Choose the facade
(`mod x;` or `pub(crate) mod x;` + `pub use x::Item;`) or the namespace (`pub mod x;` alone).

### R3 / R4: a dependency is re-exported only as named types and constants

A `pub use` whose path starts in another crate may name types and constants. It may not be a glob
(`pub use dep::*`), and it may not name a function or a module. Callers import those from the crate that
defines them (`hierarchy.md`: `src/` has no module that re-exports a crate).

```rust
// Good
pub use thyllore_scene_core::ScalarParam;

// Bad: R3 glob, R4 function
pub use thyllore_scene_core::*;
pub use thyllore_scene_core::find_scalar_param;
```

R4 does not see a function that reaches a `pub use` through a private `use dep::func;` in a child module.
That shape is still forbidden; do not rely on the test to catch it.

### R5: `components` and `systems` are never public in `crates/`

The data / logic split of a domain crate (`ecs-architecture.md`) is file layout. Declare them `mod` or `pub(crate) mod` and re-export from the domain's `mod.rs`. `src/ecs/` is not covered: there `systems`,
`component` and `resource` are the layer names.

### R6: one public path per definition

A definition or module that is already public through one module is never `pub use`d from another. Glob
re-exports count: a `pub use child::*;` lifts every public item and every public `use` of `child`.

- The path that stays is the one through the module that owns the concept (`crate::ecs::resource::BoneAxis`,
  `thyllore_avatar_api::blender_space::MatrixRows`, `crate::ecs::MAX_FRAMES_IN_FLIGHT`).
- Every other module that needs the item imports it privately with `use`, and callers write the owning path.
- Moving an item between crates removes the old path in the same change; no forwarding re-export is left
  behind (`rgb_channels` moved from color-core to scene-core, the FBX types are only
  `thyllore_file_format_core::fbx::X`).

```rust
// Bad: `MatrixRows` becomes public as `spring_preview_bridge::MatrixRows` too
pub use crate::blender_space::MatrixRows;

// Good: private import; callers keep writing `blender_space::MatrixRows`
use crate::blender_space::MatrixRows;
```

## Lints

`[workspace.lints.rust]` denies four lints, so these fail at `cargo build`, before any test runs:

| Lint | Write this instead |
|---|---|
| `unreachable_pub` | an item that no public path reaches is `pub(crate)` (or `pub(super)`), not `pub` |
| `unnameable_types` | a type that appears in a public signature or field gets a public path through its facade |
| `ambiguous_glob_reexports` | two globs that export the same name: re-export by name, or make the child `mod` private |
| `hidden_glob_reexports` | a private item that shadows a glob re-export: rename it or re-export by name |

`pub(crate) mod x;` does not remove a name from a `pub use x::*;` in the same crate; only a private `mod`
or a named re-export does.

## Adding a crate

A new workspace member writes `[lints]` / `workspace = true` in its `Cargo.toml`; the test fails for a member
without it. The patched external crates under `vendor/` do not take part.

Lints that only some crates can satisfy never go into `[workspace.lints.rust]`: Cargo cannot inherit part of
the table or override it per crate. `scratch` is the single exception to a workspace lint
(`#![allow(unreachable_pub)]`, its entry functions cannot leave `lib.rs`).

## Checklist

- [ ] Is the new module a facade or a namespace, and does the file use only that shape?
- [ ] Does `mod.rs` / `lib.rs` contain only `mod`, `use`, `pub use`?
- [ ] Is every new `pub use` the first public path to that item?
- [ ] Is every new `pub` item reachable from the crate root, and `pub(crate)` otherwise?
- [ ] Does a `pub use` of another crate name only types and constants?
