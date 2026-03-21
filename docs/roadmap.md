# Roadmap

Planned features and improvements for Thyllore Animation.

## Animation

- [ ] **Rig Propagation** — Propagate rig edits (bone transforms, constraints) across multiple animation clips automatically
- [ ] **Text-to-Motion** — Generate animation from natural language descriptions via gRPC-based ML inference

## Rendering

- [ ] **Ray Tracing (Full)** — Expand existing ray tracing support with full global illumination

## Editor

- [ ] **Transform Gizmo** — Interactive translate/rotate/scale gizmo for scene objects
- [ ] **Game View Camera** — Separate preview camera independent from the editor viewport

## Platform

- [ ] **winit 0.30 Migration** — Migrate from winit 0.29 closure-based event loop to winit 0.30+ `ApplicationHandler` trait.
  Eliminates deep nesting in main loop, enables method-based event dispatch, and improves testability.
  Requires replacing `imgui-winit-support` with `dear-imgui-winit` (ApplicationHandler-compatible alternative)
  or updating the vendored imgui-winit-support. See `ExploreHistory/20260320_ecs-refactor/` for investigation details.

## Infrastructure

- [ ] **CI / GitHub Actions** — Automated build and test pipeline
- [ ] **Installer / Packaging** — Distributable installer beyond zip archives
