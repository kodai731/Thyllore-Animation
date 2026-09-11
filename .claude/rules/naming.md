---
paths:
  - "**"
---

# Naming

## File names never repeat their directory name

The directory already namespaces its contents, so repeating it in a file name is redundant and forbidden:

- `shaders/flame/include/component.glsl`, not `shaders/flame/include/flame_component.glsl`
- `shaders/water/resolveFragment.frag`, not `shaders/water/waterResolveFragment.frag`
- `shaders/gbuffer/fragment.frag`, not `shaders/gbuffer/gbufferFragment.frag`

This applies to every tree (shaders, crates, src modules, scripts, tests): when a file only makes sense inside its
directory, name it for what it is, not for where it lives. Rust already enforces this shape (`flame::analytic::shell`,
never `flame::analytic::flame_shell`); the same rule holds for non-Rust files.

When two files in different feature directories naturally share a name (`flame/resolveFragment.frag` and
`water/resolveFragment.frag`), that is correct — disambiguation comes from the path, never from re-adding a prefix.
Consumers must therefore reference files by directory-qualified path (`passes.toml` stages, `#include` from the
`shaders/` root, spv output mirroring the source tree — see [shaders.md](shaders.md)).

## Group by feature, hierarchy first

Avoid flat directories that mix unrelated features. When files belong to a feature, give the feature its own
directory and put shared pieces in the parent:

- feature-owned: `shaders/water/`, `shaders/flame/`, `crates/thyllore-effect-core/src/flame/`
- shared across features: `shaders/include/`, `crates/thyllore-math-core/`
- a pass that is its own feature gets its own directory (`shaders/model/`), not a squat in a neighbor's
  (`pass.model` shaders do not live in `shaders/gbuffer/`)

Hierarchy takes priority over grouping style: first decide where a file sits in the tree (which feature owns it,
or that it is shared), then the name inside that directory stays short. If a name wants a prefix, that is usually
a sign the file sits in the wrong directory.
