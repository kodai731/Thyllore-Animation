# Local patches on dear-imgui-rs 0.18.0

Re-apply when updating the vendored crate.

- `src/window/mod.rs`: `Window::position_pivot` (pivot passed to `SetNextWindowPos`) and the
  imgui-rs 0.11 flag helpers `resizable`, `movable`, `collapsible`, `always_auto_resize`,
  `save_settings`, `focus_on_appearing`, `bring_to_front_on_focus`, `no_decoration`,
  `no_inputs`, `no_nav`.
- `src/window/child_window.rs`: `ChildWindow::scroll_bar` and `horizontal_scrollbar`.
- `src/io/events.rs`: `Io::input_queue_characters` (characters queued since the last frame).
