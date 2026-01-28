# Offscreen Rendering and ImGui Mouse Input Issue

**Date**: 2026-01-25
**Summary**: 3D scene was rendering to window background instead of Viewport window, and mouse input in ImGui windows affected Scene Window camera.

---

## Issue 1: Offscreen Rendering Not Working

### Symptoms
- 3D scene (grid, gizmo, models) rendered to window background
- Viewport window displayed white/blank

### Root Causes

1. **Image Layout Transition Conflict**
   - Initial transition set image to `SHADER_READ_ONLY_OPTIMAL`
   - RenderPass expected `initial_layout: UNDEFINED`
   - Vulkan attempted invalid transition

2. **Missing Layout Transitions in `transition_image_layout`**
   - `UNDEFINED` → `GENERAL` transition was not supported
   - `UNDEFINED` → `COLOR_ATTACHMENT_OPTIMAL` transition was not supported
   - GBuffer images failed to transition, causing validation errors

3. **MSAA and RenderPass Compatibility**
   - Pipelines created with MSAA 8x for main swapchain
   - Offscreen framebuffer created with 1 sample
   - RenderPass incompatibility error

4. **Format Mismatch**
   - Offscreen used `R8G8B8A8_UNORM`
   - Swapchain used `B8G8R8A8_SRGB`
   - RenderPass incompatibility

### Solutions

1. **Fixed RenderPass initial_layout** (`offscreen.rs`)
   - Changed `initial_layout` from `UNDEFINED` to `SHADER_READ_ONLY_OPTIMAL`
   - Initial transition prepares image for ImGui texture read

2. **Added missing layout transitions** (`image.rs`)
   ```rust
   (vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL) => (...)
   (vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (...)
   ```

3. **Added MSAA support to offscreen** (`offscreen.rs`)
   - Created MSAA color image (intermediate)
   - Created resolve color image (final result for ImGui)
   - Created MSAA depth image
   - Updated RenderPass with resolve attachment

4. **Used swapchain format** (`offscreen.rs`, `viewport.rs`)
   - Pass `swapchain_format` parameter to offscreen creation
   - Ensures format compatibility with pipelines

### Files Modified
- `src/vulkanr/resource/offscreen.rs`
- `src/vulkanr/resource/image.rs`
- `src/vulkanr/resource/gbuffer.rs`
- `src/app/viewport.rs`
- `src/app/init/instance.rs`
- `src/app/render.rs`
- `src/renderer/mod.rs`

---

## Issue 2: ImGui Mouse Input Affecting Scene Window

### Symptoms
- Scrolling in Debug Window caused Scene Window camera to zoom
- Left-click drag in Scene Window did not rotate camera
- Mouse wheel zoom did not work in Scene Window

### Root Cause

ImGui windows set `imgui_wants_mouse = true` when mouse is over any ImGui window, including Scene Window. This blocked:
1. Mouse click state detection (`is_left_clicked`, `is_wheel_clicked`)
2. Mouse diff calculation for camera drag
3. Camera input processing

### Solution

Check `viewport_hovered` to allow input when mouse is over Scene Window:

1. **`events.rs` - `update_mouse_input`**
   ```rust
   let allow_input = !gui_data.imgui_wants_mouse || gui_data.viewport_hovered;
   if allow_input {
       // capture mouse clicks
   }
   ```

2. **`gui_data.rs` - `update`**
   ```rust
   let allow_input = !self.imgui_wants_mouse || self.viewport_hovered;
   if !allow_input {
       self.clicked_mouse_pos = None;
       return;
   }
   // calculate mouse_diff
   ```

3. **`input_phase.rs` - `run_input_phase`**
   ```rust
   let viewport_hovered = ctx.gui_data.viewport_hovered;
   if !ctx.light_gizmo().selectable.is_selected && viewport_hovered {
       // process camera input
   }
   ```

### Files Modified
- `src/platform/events.rs`
- `src/app/gui_data.rs`
- `src/ecs/systems/phases/input_phase.rs`

---

## Key Learnings

1. **Vulkan Image Layout Transitions**
   - RenderPass automatically transitions layouts based on `initial_layout` and `final_layout`
   - Manual transitions must match RenderPass expectations
   - All transition types must be explicitly supported in barrier code

2. **Pipeline/RenderPass Compatibility**
   - Pipelines must be created with compatible RenderPass
   - MSAA sample count must match
   - Attachment formats must match

3. **ImGui Input Handling**
   - `imgui_wants_mouse` is true for ALL ImGui windows
   - Custom viewport windows need special handling
   - Check `viewport_hovered` to allow input in specific windows
