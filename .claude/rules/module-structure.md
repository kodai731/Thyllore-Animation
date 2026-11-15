---
paths:
  - "**/mod.rs"
---

# Module Structure

- Don't write definition or implementation in mod.rs
- Only module publish responsibility on mod.rs
- The public-surface rules (definitions in `mod.rs` / `lib.rs`, dual paths, dependency re-exports) are defined by
  `crates/thyllore-grpc-client/tests/module_facade.rs` and `[workspace.lints]`
