---
paths:
  - "**/mod.rs"
---

# Module Structure

- Don't write definition or implementation in mod.rs
- Only module publish responsibility on mod.rs
- The public-surface rules (definitions in `mod.rs` / `lib.rs`, dual paths, dependency re-exports, one public path per
  definition: never `pub use` an item that another module of the crate already exposes) are defined by
  `crates/thyllore-grpc-client/tests/module_facade/` and `[workspace.lints]`
