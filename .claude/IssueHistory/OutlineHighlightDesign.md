# Outline Highlight Design - G-Buffer Integration with Offscreen Rendering

## Summary
選択オブジェクトのアウトラインハイライト機能を実装するため、G-BufferフローをオフスクリーンレンダリングAに統合する設計。

## Current Architecture

### Offscreen Rendering Flow (Current)
```
1. Offscreen RenderPass開始
2. 3Dシーンをオフスクリーンテクスチャに直接描画
3. Offscreen RenderPass終了
4. Main RenderPass開始
5. ImGui描画（ビューポートにオフスクリーンテクスチャを表示）
6. Main RenderPass終了
```

### G-Buffer Flow (Existing but not integrated)
```
1. G-Buffer RenderPass開始
2. シーンをG-Bufferに描画（Position, Normal, Albedo, ObjectID）
3. G-Buffer RenderPass終了
4. Composite RenderPass開始（メインRenderPass使用）
5. G-Bufferから最終画像を合成
6. Composite RenderPass終了
```

## Target Architecture

### Integrated Flow
```
1. G-Buffer Pass (G-Buffer RenderPass)
   - Position, Normal, Albedo, ObjectIDをG-Bufferに描画
   - シーンの全メッシュを描画（push constantでObjectIDを渡す）

2. Ray Query Pass [Optional] (Compute Shader)
   - シャドウマスクを計算

3. Composite Pass (Offscreen RenderPass)
   - G-Bufferからオフスクリーンに最終画像を合成
   - エッジ検出でアウトラインを描画
   - グリッド、ギズモ、ビルボードをオフスクリーンに追加描画

4. Main Pass (Swapchain RenderPass)
   - ImGuiを描画
   - ビューポートにオフスクリーンテクスチャを表示
```

## Key Changes Required

### 1. Composite Pipeline Recreation
**Problem**: コンポジットパイプラインは現在メインRenderPass用に作成されている。オフスクリーンRenderPassで使用するには互換性のあるパイプラインが必要。

**Solution**:
- コンポジットパイプラインをオフスクリーンRenderPass用に再作成
- `raytracing.rs`の`create_pipelines`で、オフスクリーンRenderPassを使用

### 2. CompositePass Modification
**File**: `src/renderer/deferred/composite.rs`

**Changes**:
- `swapchain_extent` → オフスクリーンの`extent`を使用
- `begin_render_pass`でオフスクリーンのRenderPassとFramebufferを使用
- ビューポートとシザーをオフスクリーンサイズに合わせる

### 3. Rendering Flow Update
**File**: `src/renderer/mod.rs`

**New Flow**:
```rust
if use_gbuffer {
    // 1. G-Buffer Pass
    deferred::record_gbuffer_pass(self, command_buffer, image_index)?;

    // 2. Composite Pass (to offscreen)
    if let Some(ref offscreen) = self.data.viewport.offscreen {
        deferred::record_composite_to_offscreen(self, command_buffer, offscreen)?;
    }

    // 3. Main Pass (ImGui)
    self.begin_main_render_pass(command_buffer, image_index);
    self.record_imgui_rendering(command_buffer, draw_data)?;
    self.rrdevice.device.cmd_end_render_pass(command_buffer);
} else {
    // Fallback: original offscreen rendering
    ...
}
```

### 4. G-Buffer Size Synchronization
**Problem**: G-Bufferとオフスクリーンのサイズが異なる可能性がある。

**Solution**:
- ビューポートリサイズ時にG-Bufferもリサイズ
- または、G-Bufferをスワップチェーンサイズで作成し、コンポジット時にビューポートサイズにスケール

### 5. Selection State Synchronization
**File**: `src/renderer/deferred/mod.rs`

**Already Implemented**:
- `collect_selected_mesh_ids()`: HierarchyStateから選択メッシュIDを収集
- `update_selection()`: SelectionUBOを更新

## File Changes Overview

| File | Change Type | Description |
|------|-------------|-------------|
| `src/renderer/mod.rs` | Modify | レンダリングフロー変更 |
| `src/renderer/deferred/mod.rs` | Modify | `record_composite_to_offscreen`追加 |
| `src/renderer/deferred/composite.rs` | Modify | オフスクリーン対応 |
| `src/app/raytracing.rs` | Modify | パイプライン作成時にオフスクリーンRenderPass使用 |
| `src/vulkanr/resource/gbuffer.rs` | Add | `resize()`メソッド追加 |

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        G-Buffer Pass                            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │ Position │ │  Normal  │ │  Albedo  │ │ ObjectID │           │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘           │
└───────│────────────│────────────│────────────│──────────────────┘
        │            │            │            │
        ▼            ▼            ▼            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Composite Pass                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            Composite Shader                              │   │
│  │  - Sample G-Buffer textures                              │   │
│  │  - Apply lighting (Diffuse + Shadow)                     │   │
│  │  - Edge detection for selected objects                   │   │
│  │  - Output to offscreen framebuffer                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│  + Grid, Gizmo, Billboard rendering                            │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Offscreen Texture                           │
│  (Used by ImGui Viewport)                                       │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Main Pass                                 │
│  ImGui rendering (displays offscreen texture in viewport)       │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Steps

1. **Step 1**: コンポジットパイプラインをオフスクリーンRenderPass用に再作成
2. **Step 2**: CompositePassをオフスクリーン対応に修正
3. **Step 3**: renderer/mod.rsのレンダリングフロー変更
4. **Step 4**: G-Bufferリサイズ対応
5. **Step 5**: テストと検証

## Edge Detection Algorithm (in Composite Shader)

```glsl
bool detectOutlineEdge() {
    uint centerID = texture(objectIDSampler, inUV).r;
    if (!isSelected(centerID)) return false;

    vec2 texelSize = 1.0 / textureSize(objectIDSampler, 0);

    // 4-neighbor check
    uint leftID = texture(objectIDSampler, inUV + vec2(-texelSize.x, 0)).r;
    uint rightID = texture(objectIDSampler, inUV + vec2(texelSize.x, 0)).r;
    uint topID = texture(objectIDSampler, inUV + vec2(0, -texelSize.y)).r;
    uint bottomID = texture(objectIDSampler, inUV + vec2(0, texelSize.y)).r;

    // Edge if any neighbor has different ID
    return (leftID != centerID) || (rightID != centerID) ||
           (topID != centerID) || (bottomID != centerID);
}
```
