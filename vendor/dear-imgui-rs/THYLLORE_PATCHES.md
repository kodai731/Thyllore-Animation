# Local patches on dear-imgui-rs 0.18.0

Re-apply when updating the vendored crate.

- `src/window/mod.rs`: `Window::position_pivot` (pivot passed to `SetNextWindowPos`) and the
  imgui-rs 0.11 flag helpers `resizable`, `movable`, `collapsible`, `always_auto_resize`,
  `save_settings`, `focus_on_appearing`, `bring_to_front_on_focus`, `no_decoration`,
  `no_inputs`, `no_nav`.
- `src/window/child_window.rs`: `ChildWindow::scroll_bar` and `horizontal_scrollbar`.
- `src/io/events.rs`: `Io::input_queue_characters` (characters queued since the last frame).
- `../dear-imgui-sys/build.rs`: the MSVC branch no longer forces `opt_level(0)` in debug; the C++
  optimization level comes from Cargo's `OPT_LEVEL` so `[profile.dev.package.dear-imgui-sys]` in the
  root `Cargo.toml` applies on every platform.
- `Cargo.toml` and `../dear-imgui-build-support/Cargo.toml`: `[lib]` has `test = false` and
  `bench = false`. The unit tests `include_str!` / `include_bytes!` files by the upstream monorepo
  layout (`extensions/*`, `dear-imgui-sys` three levels up), which this tree does not have, so the
  test target cannot compile here. The integration tests under `tests/` still run.
- `tests/snapshot_contract.rs`: the `linear` / `nearest` callbacks of
  `snapshot_preserves_standard_sampler_callbacks` each count into their own static. With the
  `opt-level = 2` this workspace sets for the crate, two empty functions are merged to one address
  and both commands are read as `SetSamplerLinear`.
