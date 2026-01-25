# Std140 Alignment and Push Constants Issues

**Date**: 2026-01-25
**Status**: Resolved
**Summary**: SelectionUBOがシェーダーに正しく渡らない問題を修正。原因は (1) push constantsがシェーダーで未定義、(2) std140配列アライメントの不一致。

---

## Issue 1: Push Constants Not Defined in Shader

### Symptoms
- Debug mode 7 (ObjectID) のみ動作
- Debug mode 8, 9 が動作しない
- SelectionUBOの値がシェーダーで0になる

### Root Cause
- `composite.rs`がpush constantsでdebug mode値を送信
- シェーダーは`sceneData.debugMode`（UBO binding 4）を読んでいた
- シェーダーにpush constants定義がなかった

### Solution
シェーダーにpush constants定義を追加:

```glsl
layout(push_constant) uniform PushConstants {
    int debugMode;
} pc;
```

すべての`sceneData.debugMode`を`pc.debugMode`に変更。

---

## Issue 2: std140 Array Alignment Mismatch

### Symptoms
- mode 9でselectedCountが常に0
- mode 8でオブジェクトが選択されても色が変わらない

### Root Cause
**std140レイアウト規則**: 配列の各要素は16バイト（vec4サイズ）にパディングされる。

| Side | Definition | Size |
|------|------------|------|
| Shader | `uint selectedIDs[32]` | 32 × 16 = **512 bytes** |
| Rust | `[u32; 32]` | 32 × 4 = **128 bytes** |

`selectedCount`のオフセットが異なるため、シェーダーが正しい位置を読めなかった。

### Solution

**Rust側** (`src/vulkanr/descriptor/composite.rs`):
```rust
#[repr(C)]
pub struct SelectionUBO {
    pub selected_ids: [[u32; 4]; MAX_SELECTED_OBJECTS],  // 32 * 16 = 512 bytes
    pub selected_count: u32,
    pub _padding: [u32; 3],
}

// IDを格納する際:
ubo.selected_ids[i] = [id, 0, 0, 0];
```

**シェーダー側** (`shaders/compositeFragment.frag`):
```glsl
layout(std140, binding = 6) uniform SelectionData {
    uvec4 selectedIDs[32];
    uint selectedCount;
    uint _pad0;
    uint _pad1;
    uint _pad2;
} selection;

// アクセス:
selection.selectedIDs[i].x  // .x で最初の要素を取得
```

---

## std140 Layout Rules Summary

| Type | Base Alignment | Effective Size |
|------|---------------|----------------|
| `int`, `uint`, `float` | 4 bytes | 4 bytes |
| `vec2` | 8 bytes | 8 bytes |
| `vec3`, `vec4` | 16 bytes | 16 bytes |
| `mat4` | 16 bytes | 64 bytes |
| **Array element** | **Round up to vec4 (16 bytes)** | **16 bytes per element** |

### Key Point
`uint array[N]` in std140 = N × 16 bytes, NOT N × 4 bytes.

---

## Files Modified

| File | Change |
|------|--------|
| `shaders/compositeFragment.frag` | push constants定義追加、uvec4配列に変更 |
| `src/vulkanr/descriptor/composite.rs` | SelectionUBO構造体を[[u32; 4]; 32]に変更 |

---

## Related Issues
- `StencilOutlineHighlight.md` - 以前試したStencilアプローチ（不採用）
- `OutlineHighlightDesign.md` - G-Buffer統合設計

## Final Result
Object ID Buffer + Edge Detection方式でアウトラインハイライトが正常に動作。
