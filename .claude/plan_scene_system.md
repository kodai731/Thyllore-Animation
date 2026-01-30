# Scene File System Design

## Overview

Unity の .scene や Blender の .blend に相当するシーンファイルシステムを実装する。
RON (Rusty Object Notation) 形式を採用し、serde によるシリアライズを行う。

**設計方針: Option B（別ファイル参照）**
- シーンファイルはアニメーションクリップを参照のみ
- アニメーションクリップは個別ファイルとして保存
- Unity と同様のモジュラー設計

## File Format

### 拡張子
- `.scene.ron` - シーンファイル
- `.anim.ron` - アニメーションクリップファイル

### ファイル構成
```
assets/
├── scenes/
│   ├── default.scene.ron           # デフォルトシーン（起動時に読み込み）
│   └── my_project.scene.ron        # ユーザー作成シーン
│
├── animations/
│   ├── walk_edited.anim.ron        # 編集済みアニメーション
│   ├── run_edited.anim.ron
│   └── custom_anim.anim.ron
│
└── models/
    └── stickman/
        └── stickman.glb            # 元モデル（元アニメーション含む）
```

## Data Structures

### 1. SceneFile (シーンファイル)

```rust
#[derive(Serialize, Deserialize)]
pub struct SceneFile {
    pub version: u32,                              // フォーマットバージョン (1)
    pub metadata: SceneMetadata,
    pub model: ModelReference,                     // モデル参照
    pub animation_clips: Vec<AnimationClipRef>,    // アニメーション参照リスト
    pub current_clip: Option<String>,              // 現在選択中のクリップ名
    pub camera: CameraState,
    pub timeline: TimelineConfig,
    pub editor: EditorState,
}
```

### 2. SceneMetadata

```rust
#[derive(Serialize, Deserialize)]
pub struct SceneMetadata {
    pub name: String,
    pub created_at: String,              // ISO 8601
    pub modified_at: String,
}
```

### 3. ModelReference

```rust
#[derive(Serialize, Deserialize)]
pub struct ModelReference {
    pub path: String,                    // "models/stickman/stickman.glb"
    pub transform: TransformData,
}

#[derive(Serialize, Deserialize)]
pub struct TransformData {
    pub position: [f32; 3],
    pub rotation: [f32; 4],              // Quaternion (x, y, z, w)
    pub scale: [f32; 3],
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}
```

### 4. AnimationClipRef (アニメーション参照)

```rust
#[derive(Serialize, Deserialize)]
pub struct AnimationClipRef {
    pub path: String,                    // "animations/walk_edited.anim.ron"
}
```

### 5. AnimationClipFile (アニメーションファイル)

```rust
#[derive(Serialize, Deserialize)]
pub struct AnimationClipFile {
    pub version: u32,
    pub clip: EditableAnimationClip,     // 既存の構造体をそのまま使用
}
```

### 6. CameraState

```rust
#[derive(Serialize, Deserialize)]
pub struct CameraState {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: [0.0, 1.5, 5.0],
            direction: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            fov: 45.0,
        }
    }
}
```

### 7. TimelineConfig

```rust
#[derive(Serialize, Deserialize)]
pub struct TimelineConfig {
    pub current_time: f32,
    pub playing: bool,
    pub looping: bool,
    pub speed: f32,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            current_time: 0.0,
            playing: false,
            looping: true,
            speed: 1.0,
        }
    }
}
```

### 8. EditorState

```rust
#[derive(Serialize, Deserialize)]
pub struct EditorState {
    pub selected_bone_id: Option<u32>,
    pub curve_editor_open: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_bone_id: None,
            curve_editor_open: false,
        }
    }
}
```

## Implementation Structure

```
src/
├── scene/                              # 新規モジュール
│   ├── mod.rs
│   ├── format.rs                       # SceneFile, AnimationClipFile 等
│   ├── scene_loader.rs                 # シーン読み込み
│   ├── scene_saver.rs                  # シーン保存
│   ├── clip_loader.rs                  # アニメーションクリップ読み込み
│   └── clip_saver.rs                   # アニメーションクリップ保存
```

## Key Functions

### シーン保存
```rust
pub fn save_scene(
    path: &Path,
    world: &World,
    model_path: &str,
) -> Result<(), SceneError>
```

### シーン読み込み
```rust
pub fn load_scene(
    path: &Path,
    world: &mut World,
    graphics: &mut GraphicsResources,
) -> Result<(), SceneError>
```

### アニメーションクリップ保存
```rust
pub fn save_animation_clip(
    path: &Path,
    clip: &EditableAnimationClip,
) -> Result<(), SceneError>
```

### アニメーションクリップ読み込み
```rust
pub fn load_animation_clip(
    path: &Path,
) -> Result<EditableAnimationClip, SceneError>
```

### デフォルトシーン確認
```rust
pub fn find_default_scene() -> Option<PathBuf> {
    let path = Path::new("assets/scenes/default.scene.ron");
    if path.exists() { Some(path.to_path_buf()) } else { None }
}
```

## File Examples

### default.scene.ron

```ron
(
    version: 1,
    metadata: (
        name: "Default Scene",
        created_at: "2026-01-29T12:00:00Z",
        modified_at: "2026-01-29T15:30:00Z",
    ),
    model: (
        path: "models/stickman/stickman.glb",
        transform: (
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        ),
    ),
    animation_clips: [
        (path: "animations/walk_edited.anim.ron"),
        (path: "animations/idle_edited.anim.ron"),
    ],
    current_clip: Some("walk_edited"),
    camera: (
        position: [0.0, 1.5, 5.0],
        direction: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        fov: 45.0,
    ),
    timeline: (
        current_time: 0.0,
        playing: false,
        looping: true,
        speed: 1.0,
    ),
    editor: (
        selected_bone_id: None,
        curve_editor_open: false,
    ),
)
```

### animations/walk_edited.anim.ron

```ron
(
    version: 1,
    clip: (
        id: 1,
        name: "walk_edited",
        duration: 1.0,
        tracks: {
            0: (
                bone_id: 0,
                bone_name: "hip",
                translation_x: (
                    id: 1,
                    property_type: TranslationX,
                    keyframes: [
                        (id: 1, time: 0.0, value: 0.0),
                        (id: 2, time: 0.5, value: 0.1),
                        (id: 3, time: 1.0, value: 0.0),
                    ],
                ),
                // ... other curves
            ),
            // ... other bones
        },
        source_path: Some("models/stickman/stickman.glb"),
    ),
)
```

## Startup Flow

```
1. Application Start
2. Check assets/scenes/default.scene.ron exists?
   ├─ Yes:
   │   1. Load scene file
   │   2. Load model from model.path
   │   3. Load each animation clip from animation_clips[].path
   │   4. Set current_clip
   │   5. Restore camera, timeline, editor state
   │
   └─ No:
       1. Load hardcoded default model (current behavior)
       2. Create editable clips from model animations
```

## Save Flow

### Scene Save (Ctrl+S)
```
1. Collect current state from World
2. Save each dirty animation clip to animations/*.anim.ron
3. Save scene file with references
```

### Animation Clip Save
```
1. Get EditableAnimationClip from EditableClipManager
2. Wrap in AnimationClipFile
3. Write to animations/{clip_name}.anim.ron
```

## Implementation Phases

### Phase 1: データ構造と基本IO
1. `src/scene/` モジュール作成
2. `format.rs` - 全データ構造定義
3. `clip_saver.rs` - アニメーションクリップ保存
4. `clip_loader.rs` - アニメーションクリップ読み込み

### Phase 2: シーンIO
1. `scene_saver.rs` - シーン保存
2. `scene_loader.rs` - シーン読み込み
3. 起動時のデフォルトシーン読み込み

### Phase 3: UI統合
1. ImGui メニューに Save/Load 追加
2. Ctrl+S ショートカット
3. アニメーションクリップの個別保存UI

### Phase 4: 改善
1. 最近開いたファイル履歴
2. 未保存変更の警告
3. 自動保存（オプション）

## Path Resolution

シーンファイルからの相対パスを解決する：

```rust
pub struct PathResolver {
    scene_dir: PathBuf,      // assets/scenes/
    project_root: PathBuf,   // プロジェクトルート
}

impl PathResolver {
    pub fn resolve_model(&self, path: &str) -> PathBuf {
        // "models/stickman/stickman.glb" -> "assets/models/stickman/stickman.glb"
        self.project_root.join("assets").join(path)
    }

    pub fn resolve_animation(&self, path: &str) -> PathBuf {
        // "animations/walk.anim.ron" -> "assets/animations/walk.anim.ron"
        self.project_root.join("assets").join(path)
    }
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum SceneError {
    IoError(std::io::Error),
    ParseError(ron::Error),
    ModelNotFound(String),
    AnimationNotFound(String),
    VersionMismatch { expected: u32, found: u32 },
}
```

## Notes

- EditableAnimationClip は既に Serialize/Deserialize を実装済み
- ron クレートは既に Cargo.toml に含まれている
- アニメーションクリップは編集時に自動保存 or 明示的保存を選択可能
- バージョン番号でフォーマット互換性を管理
- source_path フィールドで元モデルとの関連を追跡
