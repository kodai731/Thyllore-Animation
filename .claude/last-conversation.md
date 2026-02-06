# Last Conversation: ECS重大違反リファクタリング計画

## 状況
- `/check-ecs` を実行し、ECSアーキテクチャの準拠チェックを行った
- 5つの重大な違反を発見
- ユーザーは「実用的アプローチ」を選択（複雑なロジックのみ移動、シンプルなアクセサは残す）
- 計画を策定済み、**実装は未着手**

## 次のアクション
以下の計画に従って Step 1 から実装を開始する。

---

# ECS重大違反リファクタリング計画

## 方針
実用的アプローチ: 複雑なビジネスロジック（ID採番、フィルタリング、計算、I/O）をシステム関数に移動。シンプルなアクセサ・ビルダーはデータ構造に残す。

## 実装順序（影響範囲が小さい順）

---

### Step 1: MeshData.validate() + interleave.rs のバッファ生成関数

**新規ファイル**: `src/ecs/systems/mesh_systems.rs`

**移動するもの**:
- `mesh_data.rs:94-112` の `validate()` → `validate_mesh_data(mesh: &MeshData)`
- `interleave.rs:38-59` の `create_interleaved_buffer()` → そのままフリー関数として移動
- `interleave.rs:61-80` の `write_attribute_value()` → private ヘルパーとして移動
- `interleave.rs:82-87` の `write_f32_array()` → private ヘルパーとして移動
- `interleave.rs:90-107` のテスト → mesh_systems.rs のテストモジュールに移動

**残すもの**:
- `MeshData`: new(), insert_attribute(), with_inserted_attribute(), set_indices(), with_indices(), attribute(), attribute_ids(), indices(), topology(), vertex_count(), index_count(), has_attribute() (全てビルダー/アクセサ)
- `VertexLayout`: 構造体 + from_mesh_data(), from_attributes() (コンストラクタ)

**変更ファイル**:
| ファイル | 変更 |
|---|---|
| `src/ecs/systems/mesh_systems.rs` | 新規作成 |
| `src/ecs/systems/mod.rs` | `mod mesh_systems; pub use mesh_systems::*;` 追加 |
| `src/ecs/component/mesh/mesh_data.rs` | validate() メソッド削除 |
| `src/ecs/component/mesh/interleave.rs` | create_interleaved_buffer + ヘルパー + テスト削除 |
| `src/ecs/component/mesh/mod.rs` | `create_interleaved_buffer` の再エクスポート削除 |
| `src/vulkanr/resource/buffer_registry.rs` | import パスを systems に変更 |

---

### Step 2: ConstraintSet のビジネスロジック移動

**新規ファイル**: `src/ecs/systems/constraint_set_systems.rs`

**移動するメソッド** (4個):
- `add_constraint()` L24-41 → `constraint_set_add(set, constraint, priority) -> ConstraintId`
- `remove_constraint()` L43-47 → `constraint_set_remove(set, id) -> bool`
- `find_by_bone()` L63-68 → `constraint_set_find_by_bone(set, bone_id) -> Vec<&ConstraintEntry>`
- `enabled_constraints()` L70-75 → `constraint_set_enabled(set) -> Vec<&ConstraintEntry>`

**残すメソッド** (3個): new(), find_constraint(), find_constraint_mut()

**フィールド公開**: `next_id: ConstraintId` を `pub` に変更

**呼び出し箇所の更新**:
| ファイル | 行 | 変更 |
|---|---|---|
| `src/ecs/systems/constraint_edit_systems.rs` | L36 | `set.add_constraint(...)` → `constraint_set_add(set, ...)` |
| `src/ecs/systems/constraint_edit_systems.rs` | L55 | `set.remove_constraint(...)` → `constraint_set_remove(set, ...)` |
| `src/ecs/systems/constraint_solve_systems.rs` | L17 | `.enabled_constraints()` → `constraint_set_enabled(...)` |
| `src/ecs/systems/constraint_gizmo_systems.rs` | L41 | `.enabled_constraints()` → `constraint_set_enabled(...)` |
| `src/ecs/systems/debug_constraint_systems.rs` | L184,222,288 | `.add_constraint(...)` → `constraint_set_add(...)` |
| `src/app/model_loader.rs` | L938 | `.add_constraint(...)` → `constraint_set_add(...)` |
| `tests/ecs_tests.rs` | 7箇所 | `.add_constraint(...)` → `constraint_set_add(...)` + import追加 |

---

### Step 3: ClipSchedule のビジネスロジック移動

**新規ファイル**: `src/ecs/systems/clip_schedule_systems.rs`

**移動するメソッド** (9個):
- `add_instance()` L23-33 → `clip_schedule_add_instance(schedule, source_id, duration)`
- `remove_instance()` L35-47 → `clip_schedule_remove_instance(schedule, instance_id)`
- `active_instances_at()` L53-66 → `clip_schedule_active_instances(schedule, time)`
- `create_group()` L68-73 → `clip_schedule_create_group(schedule, name)`
- `remove_group()` L75-77 → `clip_schedule_remove_group(schedule, group_id)`
- `add_instance_to_group()` L79-91 → `clip_schedule_add_to_group(schedule, group_id, instance_id)`
- `remove_instance_from_group()` L93-101 → `clip_schedule_remove_from_group(schedule, group_id, instance_id)`
- `find_group_for_instance()` L103-110 → `clip_schedule_find_group(schedule, instance_id)`
- `effective_instance_weight()` L112-128 → `clip_schedule_effective_weight(schedule, instance_id)`

**残すメソッド** (2個): new(), first_instance()

**フィールド公開**: `next_instance_id`, `next_group_id` を `pub` に変更

**呼び出し箇所の更新**:
| ファイル | 行 | 変更 |
|---|---|---|
| `src/app/model_loader.rs` | L988 | `.add_instance(...)` → `clip_schedule_add_instance(...)` |
| `src/platform/events.rs` | L698 | `.add_instance(...)` → `clip_schedule_add_instance(...)` |
| `src/ecs/systems/timeline_systems.rs` | L316,337,345,353,361 | 5箇所更新 |
| `src/ecs/systems/animation_playback_systems.rs` | L160,178 | 2箇所更新 |
| `src/platform/ui/clip_track_snapshot.rs` | L63 | `.find_group_for_instance(...)` → `clip_schedule_find_group(...)` |

---

### Step 4: ClipLibrary のビジネスロジック移動

**新規ファイル**: `src/ecs/systems/clip_library_systems.rs`

**移動するメソッド** (8個):
- `create_from_imported()` L48-61 → `clip_library_create_from_imported(lib, clip, bone_names)`
- `create_empty()` L63-71 → `clip_library_create_empty(lib, name)`
- `register_clip()` L73-83 → `clip_library_register_clip(lib, clip)`
- `to_playable_clip()` L134-141 → `clip_library_to_playable(lib, id)`
- `sync_dirty_clips()` L169-188 → `clip_library_sync_dirty(lib)`
- `clip_names()` L190-195 → `clip_library_clip_names(lib)`
- `save_to_file()` L197-226 → `clip_library_save_to_file(lib, id, path)`
- `load_from_file()` L228-253 → `clip_library_load_from_file(lib, path)`

**残すメソッド**: new(), clear(), clear_editable(), get(), get_mut(), get_source(), get_source_mut(), get_anim_clip_id_for_source(), find_source_id_for_anim_clip(), remove(), is_dirty(), mark_clean(), mark_dirty(), dirty_clip_ids(), all_clip_ids(), clip_count()

**フィールド公開**: `source_clips`, `dirty_sources`, `next_source_id`, `source_to_anim_id` を `pub` に変更

**呼び出し箇所の更新**:
| ファイル | 変更 |
|---|---|
| `src/app/model_loader.rs` | create_from_imported の呼び出し更新 |
| `src/platform/events.rs` | create_empty, register_clip の呼び出し更新 |
| `src/app/init/instance.rs` | register_clip の呼び出し更新 |
| `src/ecs/systems/frame_runner.rs` L97 | sync_dirty_clips → clip_library_sync_dirty |
| `src/platform/ui/clip_browser_window.rs` | clip_names → clip_library_clip_names |
| `src/platform/ui/timeline_window.rs` | clip_names → clip_library_clip_names |
| `src/scene/scene_io.rs` | clip_names, save_to_file の呼び出し更新 |

---

## 検証手順

各Step完了後:
1. `cargo build` でコンパイル確認
2. `cargo test` でリグレッション確認（test_shader_count_matches の既知失敗は無視）

全Step完了後:
1. `cargo build` で最終確認
2. `cargo test` で全テスト確認
3. `/check-ecs` で違反が解消されたか確認
