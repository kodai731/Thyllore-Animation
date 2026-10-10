---
paths:
  - "**"
---

# Review Checklist

Before opening a PR or finishing an edit, run the branch diff through these 10 items. Each item's detailed rules live in the existing rule files listed at the end of each line — those are the single source of truth, so this checklist does not repeat them.

1. `mod.rs` / `lib.rs` hold `mod` declarations and re-exports only; logic lives in sibling files → `ecs-architecture.md`, `module-structure.md`
2. Generic code (`src/app/`, `ecs/systems/phases/`, `event_dispatch`, `vulkan-core`, `scene`) never names a feature; features register through hooks → `hierarchy.md` "Feature isolation"
3. No central list / match / if-chain that must be edited when a feature is added; collect from registrations or derives → `hierarchy.md` "Hook or command?"
4. No string-keyed lookups (names compared as text, prefix slicing, Debug strings); resolve through typed / generated APIs → e.g. `HumanoidRole::from_unity_name` / `index` / `mirrored` from `crates/thyllore-avatar-core/data/humanoid_rig.toml`
5. Files under a crate's `tests/` start with `#![cfg(test)]`; `tests.rs` holds only `#[cfg(test)]` items; a module's tests live with that module → `testing.md`
6. File names state the role and never repeat the directory name → `naming.md`
7. One fact, one definition: rig facts in `humanoid_rig.toml`, generic math in `thyllore-math-core`, constants as named consts → `hierarchy.md` "Where does a new file go?"
8. Split oversized files and structs with many fields into sub-modules / sub-structs → `coding.md` §5
9. UI interactions are self-applying `UiCommand`s sent with `World::send_command`, not new central event enums; one-off work runs where it is triggered → `hierarchy.md` "Hook or command?"
10. No hard-coded tuning numbers and no golden-text comparisons; pin facts at their declaration → `hierarchy.md`, `ui.md`
