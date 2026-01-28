# Object ID Buffer Outline Highlight

**Date**: 2026-01-25
**Status**: Resolved
**Summary**: 選択オブジェクトのアウトラインハイライトをObject ID Buffer + Edge Detection方式で実装。

---

## Overview

Hierarchyウィンドウで選択されたオブジェクトに対し、オレンジ色のアウトラインを表示する機能。

---

## Architecture

### Rendering Flow

```
1. G-Buffer Pass
   - Position, Normal, Albedo, ObjectID を G-Buffer に描画
   - 各メッシュに mesh_id (1-based) を push constant で渡す

2. Composite Pass
   - G-Buffer テクスチャをサンプリング
   - SelectionUBO から選択中の mesh_id リストを取得
   - Edge Detection でアウトラインを検出・描画
```

### Edge Detection Algorithm

```glsl
bool detectOutlineEdge() {
    vec2 texelSize = 1.0 / vec2(textureSize(objectIDSampler, 0));
    uint centerID = texture(objectIDSampler, fragTexCoord).r;
    bool centerSelected = isSelected(centerID);

    // 8方向の隣接ピクセルをチェック
    for (int dy = -1; dy <= 1; dy++) {
        for (int dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) continue;

            vec2 offset = vec2(float(dx), float(dy)) * texelSize * OUTLINE_WIDTH;
            uint neighborID = texture(objectIDSampler, fragTexCoord + offset).r;
            bool neighborSelected = isSelected(neighborID);

            // 選択状態が異なる境界 = エッジ
            if (centerSelected != neighborSelected) {
                return true;
            }
        }
    }
    return false;
}
```

### Selection Check

```glsl
bool isSelected(uint id) {
    if (id == 0u) return false;  // 背景は除外
    for (uint i = 0u; i < selection.selectedCount; i++) {
        if (selection.selectedIDs[i].x == id) return true;
    }
    return false;
}
```

---

## Key Files

| File | Role |
|------|------|
| `shaders/gbufferFragment.frag` | ObjectIDをG-Bufferに出力 |
| `shaders/compositeFragment.frag` | Edge Detection + アウトライン描画 |
| `src/vulkanr/descriptor/composite.rs` | SelectionUBO定義・更新 |
| `src/renderer/deferred/mod.rs` | 選択メッシュID収集 |
| `src/renderer/deferred/gbuffer.rs` | mesh_idをpush constantで渡す |

---

## Data Structures

### SelectionUBO (Rust)

```rust
pub const MAX_SELECTED_OBJECTS: usize = 32;

#[repr(C)]
pub struct SelectionUBO {
    pub selected_ids: [[u32; 4]; MAX_SELECTED_OBJECTS],  // std140: 各要素16バイト
    pub selected_count: u32,
    pub _padding: [u32; 3],
}
```

### SelectionUBO (GLSL)

```glsl
layout(std140, binding = 6) uniform SelectionData {
    uvec4 selectedIDs[32];
    uint selectedCount;
    uint _pad0;
    uint _pad1;
    uint _pad2;
} selection;
```

---

## Debug Modes

| Mode | Name | Description |
|------|------|-------------|
| 7 | ObjectID | 各メッシュを異なる色で表示 |
| 8 | SelectionView | 選択メッシュ=オレンジ、他=グレー、背景=黒 |
| 9 | SelectionUBO | selectedCountを可視化（グレースケール） |

---

## Selection Flow

```
HierarchyState.multi_selection (選択エンティティ)
    ↓
collect_selected_mesh_ids()
    ↓ Entity → MeshRef → MeshAsset → graphics_mesh_index → mesh_id
Vec<u32> (選択メッシュIDリスト)
    ↓
update_selection()
    ↓
SelectionUBO (GPU Buffer)
    ↓
Composite Shader で isSelected() チェック
```

---

## Related Issues

- `Std140AlignmentAndPushConstants.md` - std140アライメント問題の詳細
- `OutlineHighlightDesign.md` - G-Buffer統合設計
