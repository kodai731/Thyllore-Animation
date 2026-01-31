# Timeline Window UI 設計リファレンス

各エンジン・ツールのタイムライン/アニメーションウィンドウUIを調査し、複数モデル・クリップ対応の設計材料としてまとめたもの。

---

## 1. Unity Timeline Window

### トラック/レイヤー構造
PlayableDirector コンポーネントを核とする。Timeline Asset（データ）と Timeline Instance（シーン上のバインディング）が分離。

トラックタイプ:
- **Animation Track**（青）: AnimationClip のインポート/録画
- **Activation Track**（緑）: GameObject の有効/無効制御
- **Audio Track**: AudioSource 制御
- **Control Track**（ターコイズ）: ネストされた Timeline や ParticleSystem
- **Signal Track**: 外部システムとの通信

各トラックは色分けとアイコンで識別。

### バインディング機構
- **TrackAsset → Scene Object** のマッピングが PlayableDirector に保存
- 2種類:
  1. **GenericBinding**（トラックレベル）: `SetGenericBinding(trackAsset, targetObject)`
  2. **ExposedReference**（クリップレベル）: Control Track のクリップ用
- バインディングは Timeline Instance（シーン側）に保存。Asset 側ではない
- Inspector の Bindings エリアでトラック一覧とバインド先 GameObject を表示

### ビジュアルレイアウト
左右分割:
- **左パネル**: トラック名、タイプアイコン、バインド先 GameObject 名、Record/Mute/Lock ボタン
- **右パネル**: タイムルーラー上にクリップ配置。ドラッグ、トリミング、ブレンド可能
- **上部**: 再生コントロールとプレイヘッド

### クリップ表現
- 色付き矩形ブロック
- 配置時に2本のガイドライン（開始/終了時間）
- クリップ重なりでブレンド領域が自動生成

### マルチオブジェクト対応
- 複数 GameObject をそれぞれ別トラックにバインド
- Track Group / Track Sub-Group で階層的整理

---

## 2. Unity Animation Window

### プロパティカーブ
- **Dopesheet モード**: キーフレームのみ表示。ダイヤモンド形マーカー
- **Curves モード**: 値の補間をカーブとして描画。プロパティごとに色分け
- Transform の X/Y/Z は3本セットで追加

### キーフレーム編集
- ダブルクリックで追加、ドラッグで移動、複数選択
- タンジェント制御: Clamped Auto / Free Smooth
- Rotation は内部 Quaternion、UI では Euler Angles 表示

### Record モード
- プロパティ変更でキーフレーム自動記録
- 新バインディングも自動的に AnimationClip に保存

### 設計パターン
- Hierarchy / Inspector と連携: 選択中 GameObject のアニメーションデータが自動表示
- プロパティの展開/折りたたみで階層操作

---

## 3. Unreal Engine Sequencer

### トラック構造
Level Sequence Asset と Level Sequence Actor の2層構造。

主要トラックタイプ:
- **Object Binding Track**: Actor バインド → Property Track 自動生成
  - Static Mesh Actor → Transform Track
  - Skeletal Mesh Actor → Transform + Animation Track
  - Camera Actor → Transform + Camera Component
  - Light Actor → Light Component
- **Camera Cut Track**: アクティブカメラ切り替え
- **Property Track**: Actor プロパティのアニメーション
- **Folder Track**: トラック整理用
- **Subscenes Track**: サブシーケンス

### バインディング機構
- Actor 追加で Object Binding Track が自動生成
- 複数 Actor を単一トラックにバインド可能（黄色シェブロン + バインド数表示）
- 内部クラス:
  - `FMovieSceneTrackEditor`: カスタムセクションデータ生成
  - `ISequencerSection`: セクション UI 描画
  - `FMovieSceneEvalTemplate`: ランタイム評価ロジック

### ビジュアルレイアウト
- **左パネル（Outliner）**: トラックリスト、検索/フィルタ、ドラッグ並べ替え
- **右パネル**: Section を時間軸上に表示
  - 開始マーカー（緑）/ 終了マーカー（赤）で再生範囲表示
  - Selection Range で任意領域定義

### セクションとクリップ
- Section = トラック内の評価時間範囲。無限長/有限長
- Active Section（緑枠）が新規キーフレーム受け取り先
- 重なりで自動ブレンドカーブ生成
- カスタムカラー設定可能

### トラック状態管理
- **Locked**: 編集防止（赤枠）
- **Pinned**: 上部固定
- **Muted**: 非アクティブ
- **Solo**: 他トラック全ミュート

### パフォーマンス設計（4.26+）
- フェーズ分離: Spawn → Instantiation → Evaluation → Finalization
- Spawn/Instantiation は必要時のみ実行
- Evaluation/Finalization は毎フレーム実行

---

## 4. Blender NLA（Non-Linear Animation）Editor

### アクションストリップ
- **Action Strip**: アクションデータのコンテナ。「Push Down」で Strip に変換
- 3タイプ:
  1. **Action Clip**: アクションデータ
  2. **Transition**: アクション間の補間
  3. **Meta**: 複数ストリップのグループ
- インスタンス化可能（Linked Duplicate）

### トラック構造
- **下から上に**ブレンド（下層がベース、上層がオーバーライド）
- 最上部オレンジヘッダーが編集中アクション
- Tweak Mode（Tab）で任意ストリップを編集可能にする

### ブレンド
- ブレンドモード: Replace / Multiply / Subtract / Add
- Auto Blend In/Out: 重なり領域で線形ランプ自動生成
- Animated Influence: ストリップ影響度をキーフレームアニメーション可能

---

## 5. Blender Dope Sheet / Timeline

### チャンネル構造
階層的チャンネル表示:
- Top Level: F-curve データブロック全体のキーフレーム集約
- Mid Level: オブジェクトごとの中間集約
- Low Level: 個別 F-curve とキーフレーム

### キーフレーム可視化
- 形状でハンドルタイプ表現（丸: Auto Clamped、四角: Vector）
- 隣接キーフレーム間で値変化なしなら灰色/黄色ラインで接続

### Summary Row
- 全オブジェクトのキーフレーム集約行を最上部に表示
- 階層展開で詳細確認可能
- Summary キーフレーム選択で配下全要素が選択

---

## 6. Adobe After Effects / Premiere Pro

### After Effects レイヤー構造
- Composition が作業単位。各 Composition が独自 Timeline を持つ
- レイヤーベース設計: 各レイヤーは1つのフッテージを保持
- レイヤー順序が前後関係を決定

### プロパティ階層（Twirling）
- 各レイヤーに Transform グループ（Position, Scale, Rotation, Opacity, Anchor Point）
- 三角アイコンで展開/折りたたみ
- Shape Layer では無限に深い階層が可能

### キーフレームシステム
- **Stopwatch**: プロパティごとに存在。有効化でキーフレーム記録開始
- Layer Bar モード: レイヤー持続期間を帯で表示
- Graph Editor モード: カーブベースの精密制御
- Nesting / Pre-compose: Composition のネストで複雑さ管理

### Premiere Pro トラック構造
- マルチトラック設計: 複数ビデオ/オーディオトラックにクリップ配置
- ビデオトラックの上下でレイヤー順序決定
- Ripple Edit Tool: トリミング時に後続クリップ自動移動

---

## 7. Pixar Presto

### 概要
- Pixar 社内専用。『Brave』（2012）以降使用
- 外部販売なし、詳細 UI ドキュメントは限定的

### 判明している特徴
- Maya/3ds Max に似たインターフェース
- **ロール別ビュー**: アニメーター/リガー/モデラーが各自の作業情報のみ表示
- **リアルタイム再生**: ファー、ライト、シャドウ含む複数キャラクターの GPU リアルタイムレンダリング
- **タイムラインベースアニメーション**: スカルプトブラシ + レイヤードモディフィケーション + 調整可能フォールオフ
- USD（Universal Scene Description）の基盤

---

## 8. Maya

### 8.1 Time Slider

Maya ウィンドウ下部に水平配置される中核 UI:
- **Current Time Indicator**: グレーブロック。ドラッグでスクラブ
- **Key Ticks**: 赤色マークで選択オブジェクトのキーフレーム位置表示
- **Breakdown Ticks**: 緑色マークでブレイクダウンキー表示
- **Bookmarks**: タイムライン上部にカラータグ。シーン内の重要ポイントのフラグ
- **Range Slider**: 再生範囲制御。Animation Range（全体）と Playback Range（作業範囲）の二重構造
- **Cached Playback**（2019+）: メモリキャッシュによる高速再生。青ストライプでキャッシュ状態表示

選択連動: 選択オブジェクトに応じて Key Ticks が動的変化。

### 8.2 Graph Editor

左右2パネル構成:

**左パネル（Outliner）**:
- 選択オブジェクトのアニメーションチャネルを階層表示
- ノード展開で Translate X/Y/Z, Rotate X/Y/Z 等の個別チャネル表示
- チャネル選択で右パネルに対応カーブ表示
- 検索フィールドによるフィルタ。Outliner 分割表示可能

**右パネル（Graph View）**:
- 横軸=時間、縦軸=値。キーポイントとタンジェントハンドルの表示/編集
- 上部に Time Ruler（ドラッグでスクラブ可能）

**タンジェントタイプ**:

| タイプ | 説明 | 用途 |
|--------|------|------|
| Auto | 隣接キー値でクランプ。デフォルト | 汎用 |
| Spline | 前後キーを平均化した滑らかな補間 | 滑らかな動き |
| Linear | 直線接続 | 等速運動 |
| Clamped | Spline + Linear のハイブリッド | 歩行サイクルの滑り防止 |
| Plateau | イーズイン/アウト + 等値間フラット | 安定した動き |
| Stepped | 次のキーまで値保持 | ブロッキング |
| Flat | タンジェント水平（傾斜0度） | 頂点/谷の滞留 |
| Fixed | キー編集時タンジェント不変 | 手動調整保持 |

**主要機能**:
- **Break Tangents**: イン/アウトタンジェント個別操作（青色表示）
- **Weighted Tangents**: タンジェントハンドルの長さ（影響範囲）制御
- **Buffer Curves**: カーブのスナップショット保存/比較/スワップ
- **Breakdown Keys**: 2キー間の自動補間キー（緑色表示）。前後キー移動で比例移動
- **正規化表示**: -1〜1 に正規化して全カーブを同一スケール表示
- **Channel Box 連携**: Sync Graph Editor Display で選択チャネル同期

### 8.3 Dope Sheet

キーフレームのタイミング概観と編集に特化:

**左パネル（Outliner）**:
- 属性をタイプとオブジェクトで階層的にグループ化:
  1. Dopesheet Summary（全選択オブジェクトの全キー）
  2. Scene Summary（シーン全体）
  3. Object Groups（各オブジェクトの全キーヤブル属性）
  4. Compound Attributes（Translate, Rotate 等のグループ）
  5. Individual Attributes（個別属性）

**右パネル**:
- 横軸=時間、縦軸=Outliner 対応アイテム
- キーはカラーブロック（矩形）表示。色は階層内の位置を示す
- **Hierarchy Below**: 親ノードが子ノードのアニメーション情報をサマリー表示
- Dope Sheet / Edit > Keys / Time Slider は共通キークリップボード

### 8.4 Trax Editor（NLA）

Graph Editor / Dope Sheet より高レベルなシーケンス操作:

**クリップの種類**:
1. **Source Clip**: アニメーションカーブを含むオリジナルデータ。Visor/Outliner からのみアクセス
2. **Scheduled Clip（インスタンス）**: Source Clip のインスタンス。トラック上に配置。独自カーブを持たず Source を参照

**キャラクターセット要件**: クリップ作成にキャラクターセットが必須。

**クリップ操作**: スケール/サイクル、トリミング、ブレンド、分割/マージ、Time Warp Curves

**DG ノード構造**:
```
character ─── clipLibrary (全 Source Clip + カーブ管理)
    └── clipScheduler (Scheduled Clip + ブレンド管理)
              ├── animClip (Scheduled Clip) → Source 参照
              └── animBlendInOut (ブレンドノード)
```

### 8.5 Time Editor（Trax の後継、Maya 2017+）

キャラクターセット不要の新 NLA エディタ:

**ノード階層**:
```
timeEditor
    └── Composition (Tracks)
            ├── Track 0 (Animation Track)
            │     ├── Clip A (Animation Source 1 のインスタンス)
            │     └── Clip B (Animation Source 2 のインスタンス)
            └── Track 1 (Audio Track)
```

**Animation Source**: クリップの元データ。複数クリップが同一 Source をインスタンス参照可能（イタリック表示）

**主要機能**:
- クロスフェード（隣接クリップオーバーラップで自動生成）
- ウェイト調整、スケーリング、ループ（倍率指定）
- Time Warp / Speed Curve（非線形リタイミング）
- **Clip Layers**: クリップ上にレイヤー作成可能
- **Group Clips**: 複数クリップのグループ化
- **非破壊編集**: Animation Source は保護。リタイミング操作は Source に影響しない

**Trax Editor との違い**:

| 項目 | Trax | Time Editor |
|------|------|-------------|
| キャラクターセット | 必須 | 不要 |
| クリップレイヤー | なし | あり |
| コンポジション管理 | なし | あり |

### 8.6 Animation Layers

**レイヤーモード**:
- **Override**: 同一属性の先行レイヤーを上書き
- **Additive**: 先行レイヤーに加算
- **Override-Passthrough**: Passthrough ON/OFF で先行レイヤーの通過制御。Weight でブレンド率制御

**Weight**: 0.0〜1.0。アニメーション（時間変化）も可能。

**内部ノード**: animLayer ノード + animBlendNode 系ノード（型に応じた多数バリエーション）で実現。

### 8.7 Character Sets

アニメーション対象属性の集合体を定義する特殊セットノード:
- **サブキャラクターセット**: 階層構造（全身 > 上半身 > 腕 > 手）
- クリップ作成時、character に接続された全カーブがクリップの一部となり、clipLibrary に移動
- MEL: `character` コマンド / API: `MFnCharacter`, `MFnClip`

### 8.8 animCurve ノード体系

アニメーションカーブは8つの特殊ノードタイプ。命名は「入力型→出力型」:

**Time 入力（通常キーフレーム）**:

| ノード | 出力型 | 用途 |
|--------|--------|------|
| animCurveTL | Distance | translateX/Y/Z |
| animCurveTA | Angle | rotateX/Y/Z |
| animCurveTT | Time | 時間リマッピング |
| animCurveTU | Unitless | scale, visibility 等 |

**Unitless 入力（Set Driven Key）**: animCurveUL/UA/UT/UU

**暗黙の Time 接続**: animCurveT* の入力が未接続の場合、DG の `time` ノードへの暗黙接続を持つ。

### 8.9 DG / EG アーキテクチャ

**Dependency Graph (DG)**:
- 全てのシーン要素がノードとプラグ接続で表現
- **Dirty Propagation**: 属性変更時に影響先を dirty マーク → 再帰伝播 → 必要時に Pull 方式で再計算
- アニメーションカーブ含む全てがノード

**Evaluation Graph (EG, 2016+)**:
- DG の簡略化版（プラグレベル → ノードレベル）
- 評価モード: DG Mode / Serial Mode / **Parallel Mode**（全コア並列）
- **GPU Override**: OpenCL でデフォメーション処理を GPU 高速化
- **Custom Evaluator**: EG のサブセクション評価をオーバーライド可能

**アニメーション評価パイプライン**:
```
フレーム変更 → animCurveT* が値計算 → EG に沿って依存ノード評価
→ リグ/デフォーマ評価 → GPU Override → Viewport 2.0 レンダリング
```

### 8.10 Maya の設計パターン要約

1. **ノードグラフとしてのアニメーションデータ**: animCurve, animLayer, character, clipLibrary 等全てが DG ノード
2. **Source/Instance パターン**: ソースデータ保護 + 複数インスタンス参照
3. **選択連動型 UI**: Channel Box, Graph Editor, Dope Sheet, Time Slider 全てが選択状態で動的変化
4. **階層的チャネル組織**: オブジェクト → 属性タイプ → 個別属性
5. **非破壊編集**: Animation Source 保護、クリップレベルでの操作独立
6. **レイヤーベース合成**: Override/Additive + Weight 制御
7. **評価グラフ分離 (DG/EG)**: データフロー定義と評価実行の分離。並列処理・GPU 活用

---

## 共通設計パターン

### 1. 左右分割レイアウト（最も普遍的）

| ツール | 左パネル | 右パネル |
|--------|----------|----------|
| Unity Timeline | トラック名 + バインド先 + アイコン | クリップ配置 |
| Unity Animation | プロパティバインディング | Dopesheet / Curves |
| UE Sequencer | Outliner（トラック + フィルタ） | Section |
| Blender NLA | トラック名 + Mute/Lock/Solo | ストリップ配置 |
| Blender Dope Sheet | チャンネル階層 | キーフレーム |
| After Effects | レイヤーコントロール | タイムグラフ |
| Maya Graph Editor | チャネル階層 + 検索 | カーブ表示/編集 |
| Maya Dope Sheet | 属性グループ階層 | キーブロック |
| Maya Time Editor | オブジェクト/トラック階層 | クリップ配置 |

### 2. バインディング設計の選択肢

| アプローチ | 採用ツール | 特徴 |
|------------|------------|------|
| TrackAsset ↔ Scene Object | Unity Timeline | Asset と Instance の分離。再利用性高 |
| Object Binding Track | UE Sequencer | Actor 追加でトラック自動生成。直感的 |
| Action → Object | Blender NLA | アクションは Object に紐づく |
| Property Path | Unity Animation | Component.Property パスで直接参照 |
| Character Set | Maya Trax / Time Editor | 属性集合体に対するクリップバインド。Source/Instance 分離 |

### 3. クリップ/セクション表現

| パターン | 採用ツール |
|----------|------------|
| 色付き矩形ブロック | Unity Timeline, UE Sequencer, Blender NLA, Maya Time Editor |
| ダイヤモンド形キーフレーム | Unity Animation, Blender Dope Sheet |
| カラーブロック（矩形キー） | Maya Dope Sheet |
| レイヤーバー | After Effects |
| カーブ表示 | Unity Animation, After Effects, Blender, Maya Graph Editor |

### 4. 階層構造表現

| アプローチ | 採用ツール |
|------------|------------|
| 折りたたみツリー | UE Sequencer, After Effects, Blender, Maya |
| Track Group | Unity Timeline |
| Summary Row | Blender Dope Sheet, Maya Dope Sheet |
| Folder Track | UE Sequencer |
| サブキャラクターセット | Maya |

### 5. マルチオブジェクト対応

| アプローチ | 採用ツール |
|------------|------------|
| オブジェクトごとのトラック | Unity Timeline, UE Sequencer, Maya Time Editor |
| サブシーケンス | UE Sequencer |
| Nested Composition | After Effects, Maya Time Editor（コンポジション） |
| NLA Track Stack | Blender NLA |
| キャラクターセット | Maya Trax |

### 6. トラック状態管理（共通パターン）
- Mute: 非アクティブ化
- Solo: 単独再生
- Lock: 編集防止
- Pin: 上部固定表示

### 7. 再生・スクラブ（共通パターン）
- プレイヘッド: 赤い縦線
- タイムルーラー: 上部配置、クリック/ドラッグでスクラブ
- 再生コントロール: Play / Pause / Stop + ループ
- ズーム: 水平方向スケーリング
- 範囲マーカー: 再生範囲の開始/終了表示

---
---

# 設計案: マルチモデル・マルチクリップ対応 Timeline Window

Maya のように詳細にアニメーションを編集できる設計。
既存 ECS アーキテクチャを活かし、Entity ↔ Clip のバインディングを軸に構成する。

## アニメーション評価アーキテクチャ（リファクタリング後）

### データ所有権

```
AssetStorage (正本)
├── skeletons: HashMap<AssetId, SkeletonAsset>   ← Skeleton の唯一の所有者
├── animation_clips: HashMap<AssetId, AnimationClipAsset>
├── meshes, materials, nodes                      ← 他のアセット
└── get_skeleton_by_skeleton_id(SkeletonId) -> Option<&Skeleton>

ClipLibrary (Resource)
├── animation: AnimationSystem                    ← clips のみ保持（skeletons は参照用残存、正本ではない）
├── morph_animation: MorphAnimationSystem
├── editable_clips: HashMap<EditableClipId, EditableAnimationClip>
└── dirty_clips, editable_to_anim_id             ← 編集 ↔ 再生の同期
```

### SkeletonPose（一時計算データ）

```
BoneLocalPose { translation, rotation, scale }
SkeletonPose { skeleton_id, bone_poses: Vec<BoneLocalPose> }
```

SkeletonPose は毎フレーム一時的に生成される。
Skeleton の bone.local_transform（レストポーズ）を変更しない。

### skeleton_pose_systems（全計算ロジック）

```
create_pose_from_rest(skeleton: &Skeleton) -> SkeletonPose
sample_clip_to_pose(clip, time, skeleton, &mut pose, looping)
compute_pose_global_transforms(skeleton, pose) -> Vec<Matrix4>
compute_rest_global_transforms(skeleton) -> Vec<Matrix4>
apply_skinning(skin_data, global_transforms, skeleton, out_positions, out_normals)
```

ECS data-behavior 分離: data.rs にはデータ定義のみ、計算は system 関数で実行。

### 評価フロー

```
evaluate_animators:
  1. Animator から time, current_clip_id, looping を取得
  2. Morph アニメーション適用（変更なし）
  3. skeleton_id → assets.get_skeleton_by_skeleton_id() で Skeleton 取得
  4. let mut pose = create_pose_from_rest(skeleton)
  5. sample_clip_to_pose(clip, time, skeleton, &mut pose, looping)
  6. AnimationType に応じて:
     - Skeletal: compute_pose_global_transforms → prepare_skinned_vertices
     - Node: prepare_node_animation(nodes, skeleton, &pose, scale)
```

## 現状の課題

```
現在の構造（グローバル・シングルモデル）:

  ClipLibrary (Resource)      ← クリップの倉庫（Skeleton は AssetStorage が正本）
  AnimationPlayback (Resource) ← 再生状態。グローバルに1つ
  ModelState (Resource)        ← model_path, animation_type。グローバルに1つ
  TimelineState (Resource)     ← current_clip_id が1つだけ
  Animator (Component)         ← Entity に付くが、evaluate_animators は1体しか評価しない
```

問題:
1. TimelineState.current_clip_id が1つ → 複数 Entity のクリップを同時表示不可
2. evaluate_animators が最初の Animated Entity しか評価しない
3. ModelState がグローバル → 複数モデル不可

## 設計方針

Maya の Source/Instance + UE Sequencer の Object Binding Track を参考に、
ECS のデータ駆動設計に落とし込む。

**核心**: Entity が Animator（再生状態）と ClipSchedule（クリップ配置）の2つのコンポーネントを持つ。
Timeline Window は Animator を持つ Entity を自動的にトラックとして表示する。

## ECS データ設計

### Component（Entity に付くデータ）

```
Animator (既存 — 再生状態のみ)
├── time: f32
├── speed: f32
├── playing: bool
└── looping: bool

ClipSchedule (新規 — この Entity のクリップ配置)
└── instances: Vec<ClipInstance>
```

**責務分離の原則**:
- Animator = 「今どの時刻を再生しているか」（再生状態）
- ClipSchedule = 「どのクリップをいつ配置するか」（スケジューリング）
- 2つを同一コンポーネントに混ぜない（Single Responsibility）

**Animated** (既存マーカー): 変更なし。Animator を持つ Entity に付与。

### Resource（グローバル状態）

```
ClipLibrary (既存を拡張)
├── animation: AnimationSystem         ← clips のみ（Skeleton は AssetStorage が正本）
├── morph_animation: MorphAnimationSystem
├── source_clips: HashMap<SourceClipId, SourceClip>
├── dirty_sources, next_source_id, source_to_anim_id
└── クリップデータの倉庫。バインディングは ClipSchedule 側
```

ClipLibrary はクリップの「倉庫」のまま。Skeleton を含まない。
どの Entity がどのクリップを使うかは ClipSchedule.instances で表現。
= Maya の clipLibrary（倉庫）+ clipScheduler（配置）の分離に対応。

```
AssetStorage (正本)
├── skeletons: HashMap<AssetId, SkeletonAsset>   ← Skeleton の唯一の所有者
└── get_skeleton_by_skeleton_id(SkeletonId)      ← SkeletonId で検索
```

```
TimelineState (大幅拡張 — UI 表示状態のみ)
├── global_time: f32                   ← グローバルプレイヘッド
├── playing: bool
├── looping: bool
├── speed: f32
├── zoom_level: f32
├── scroll_offset: f32
├── scrubbing: bool
│
├── track_states: HashMap<Entity, TrackState>  ← Entity ごとのトラック展開状態
├── active_track: Option<Entity>               ← 現在操作中のトラック（＝Entity）
├── active_clip_id: Option<SourceClipId>       ← 現在編集中のクリップ
│
├── selected_keyframes: HashSet<SelectedKeyframe>
├── expanded_tracks: HashSet<BoneId>           ← ボーントラック展開（active_clip 内）
├── show_translation: bool
├── show_rotation: bool
└── show_scale: bool

TrackState
├── expanded: bool                     ← Entity トラックの展開/折りたたみ
├── muted: bool
├── solo: bool
└── locked: bool
```

```
AnimationPlayback (既存)
→ 将来的に削除候補。Animator コンポーネントに完全移行後は不要。
  現時点では互換性のため残す。
```

### ECS 責務まとめ

| 層 | 責務 | 種別 |
|---|---|---|
| Animator | 現在時刻・再生/停止・速度 | Component |
| ClipSchedule | どのクリップをいつ配置するか（instances のみ） | Component |
| ClipLibrary | SourceClip の保管・編集・同期 | Resource |
| AssetStorage | Skeleton の正本 | Resource |
| TimelineState | UI 表示状態（展開/選択/ズーム） | Resource |

## Timeline Window UI レイアウト

```
┌─ Timeline ───────────────────────────────────────────────────┐
│ [>] [||] [□]  Loop[✓]  Time: 1.25s / 3.00s  [-][+] Zoom:1.0│ ← Transport Bar
│─────────────────────────────────────────────────────────────│
│         │  0.0   0.5   1.0   1.5   2.0   2.5   3.0         │ ← Time Ruler
│         │    |     |     |  ▼  |     |     |     |          │    (▼ = Playhead)
│─────────┼───────────────────────────────────────────────────│
│ ▼ Stickman │ ██ walk_cycle ██████  ██ idle █████           │ ← Entity Track
│   M S L    │                                                │    (M=Mute S=Solo L=Lock)
│─────────┼───────────────────────────────────────────────────│
│   ▼ walk_cycle │                                            │ ← Clip Detail (展開時)
│     > Hips     │ ◆     ◆        ◆     ◆                   │    Bone Tracks
│     > Spine    │ ◆  ◆     ◆  ◆     ◆                      │    (Dopesheet モード)
│     > L_Arm    │ ◆        ◆        ◆                       │
│─────────┼───────────────────────────────────────────────────│
│ > Camera   │                                                │ ← 非 Animated Entity
│            │ (no animation)                                 │    (Animator なし = 空)
│─────────┼───────────────────────────────────────────────────│
│ ▼ Dragon  │ ██ fly_loop █████████████████                  │ ← 2体目の Entity
│   M S L   │                                                 │
└─────────────────────────────────────────────────────────────┘
```

### 左パネル（Track Headers）
- Hierarchy と同じ Entity ツリーだが、**Animator を持つ Entity のみ表示**
- Entity 名の左に展開ボタン（▼/▶）
- Entity 展開 → バインドされたクリップ一覧
- クリップ展開 → Bone Track 一覧（Dopesheet / Curve 表示切替）
- 各 Entity トラックに **Mute / Solo / Lock** ボタン

### 右パネル（Timeline Content）
- Time Ruler + Playhead（上部）
- Entity トラック行: クリップを色付き矩形ブロックで表示
  - クリップのドラッグ移動（start_time 変更）
  - クリップの右端ドラッグでトリミング（duration_override 変更）
- クリップ展開時: Bone Track 行に Dopesheet キーフレーム表示
- さらに展開: 個別 PropertyCurve のカーブ表示

### 階層構造

```
Level 0: Entity Track         ← Animator を持つ Entity
  Level 1: Clip Block         ← ClipSchedule.instances の各 ClipInstance
    Level 2: Bone Track       ← EditableAnimationClip.tracks の各 BoneTrack
      Level 3: Property Curve ← BoneTrack 内の translation_x 等
```

## ECS Systems 設計

### 新規 System

```
evaluate_all_animators (animation_playback_systems.rs を拡張)
  全 Animated Entity をイテレートし、Animator + ClipSchedule に基づいてアニメーション評価。
  現在の evaluate_animators を複数 Entity 対応に拡張。

  処理フロー:
    for (entity, animator) in world.iter_animated_entities():
      let schedule = world.get_component::<ClipSchedule>(entity)
      skeleton = assets.get_skeleton_by_skeleton_id(skel_id)
      let pose = evaluate_entity_animation(animator, schedule, skeleton, clip_library)
      let globals = compute_pose_global_transforms(skeleton, &pose)
      prepare_skinned_vertices(&globals, skeleton) / prepare_node_animation(nodes, skeleton, &pose, scale)
```

```
advance_animator_time (新規 system)
  再生中の各 Animator の time を delta_time * speed で進める。
  ループ時はクリップ duration で折り返す。

  現在は timeline_update が AnimationPlayback.time を更新し、
  sync_playback_to_animator で Animator に転写している。
  → Animator.time を直接更新する方式に変更。
```

```
sync_timeline_to_animators (frame_runner.rs)
  TimelineState.global_time を各 Animator.time に反映。
  Timeline UI のスクラブ操作時に全 Animator を同期。
```

### 既存 System の変更

```
run_timeline_phase (frame_runner.rs)
  現在: AnimationPlayback を更新 → Animator に同期
  変更: TimelineState.global_time を直接 Animator に書き込み
        AnimationPlayback への依存を段階的に除去

run_animation_phase_ecs (animation_phase.rs)
  現在: evaluate_animators（単一 Entity）
  変更: evaluate_all_animators（全 Animated Entity）
```

### UIEvent 拡張

```
現在の Timeline 系イベント → 維持

追加:
  TimelineBindClip { entity: Entity, clip_id: EditableClipId }
  TimelineUnbindClip { entity: Entity, clip_index: usize }
  TimelineMoveClipOnTrack { entity: Entity, clip_index: usize, new_start_time: f32 }
  TimelineTrimClip { entity: Entity, clip_index: usize, new_duration: f32 }
  TimelineSetActiveTrack(Entity)
  TimelineSetActiveClip(EditableClipId)
  TimelineToggleTrackMute(Entity)
  TimelineToggleTrackSolo(Entity)
  TimelineToggleTrackLock(Entity)
  TimelineExpandEntityTrack(Entity)
  TimelineCollapseEntityTrack(Entity)
```

## フレーム評価フロー（変更後）

```
run_frame:
  1. run_input_phase
  2. run_transform_phase_ecs
  3. run_timeline_phase              ← TimelineState → 全 Animator に時間同期
  4. run_animation_phase_ecs         ← 全 Animated Entity を評価
  5. run_animation_phase_gpu
  6. run_transform_phase_gpu
  7. run_render_prep_phase
```

```
run_timeline_phase 詳細:
  1. global_time を playing なら delta_time * speed で進める
  2. 全 Animated Entity の Animator.time を global_time に同期
  3. dirty クリップを sync
```

```
run_animation_phase_ecs 詳細:
  for each Animated Entity:
    1. Animator から time を取得
    2. ClipSchedule から instances を取得
    3. assets.get_skeleton_by_skeleton_id(skel_id) で不変 Skeleton 取得
    4. アクティブ instances を順に評価 → blend_poses_override / additive
    5. AnimationType に応じて prepare_skinned_vertices / prepare_node_animation
```

## 段階的移行計画

### Phase A: ClipSchedule コンポーネント導入
- ClipSchedule コンポーネントを新規定義（instances のみ）
- Animator は再生状態のみ（変更なし）
- 既存の current_clip_id を ClipSchedule.instances[0] に変換する互換レイヤー
- UI は既存のまま

### Phase B: Timeline Window マルチトラック化
- TimelineState に track_states, active_track 追加
- Timeline Window の左パネルを Entity ベースに変更
- クリップの矩形ブロック表示

### Phase C: 複数 Entity 評価
- evaluate_all_animators 実装
- AnimationPlayback 依存の除去
- 複数モデルの同時アニメーション

### Phase D: クリップ操作 UI
- クリップのドラッグ移動/トリミング
- クリップの Entity へのバインド/アンバインド（D&D）
- Mute/Solo/Lock

### Phase E: 詳細編集
- Bone Track 展開 → Dopesheet 表示
- Property Curve 展開 → Curve Editor 連携
- キーフレームの追加/移動/削除（既存機能の再接続）

---
---

# 複数アニメーションクリップシステム 詳細設計

上記の設計案を土台とし、Maya レベルの詳細アニメーション編集を実現するための
追加設計。現行コードベースの制約（ImGui ベース UI、シングルスレッド評価）を考慮しつつ、
段階的に拡張可能な設計とする。

**重要**: UI表現は Maya を参考にするが、ECS設計の厳守を第一とする。
そのため、Mayaと乖離することは認める。

## 1. Source/Instance クリップパターン

Maya の clipLibrary（Source）+ animClip（Scheduled Instance）に対応。
非破壊編集の基盤。

### データ構造

```
SourceClip (ClipLibrary に格納)
├── id: SourceClipId
├── name: String
├── duration: f32
├── editable_clip: EditableAnimationClip  ← 実際のカーブデータ
├── source_path: Option<String>           ← インポート元
└── ref_count: u32                        ← 参照カウント（GC 用）

ClipInstance (ClipSchedule.instances の要素)
├── instance_id: ClipInstanceId
├── source_id: SourceClipId               ← どの SourceClip を参照するか
├── start_time: f32                       ← タイムライン上の配置開始時刻
├── clip_in: f32                          ← ソース内の開始オフセット（トリミング左端）
├── clip_out: f32                         ← ソース内の終了オフセット（トリミング右端）
├── speed: f32                            ← 再生速度倍率（1.0 = 等速）
├── weight: f32                           ← ブレンド重み 0.0〜1.0
├── blend_mode: BlendMode                 ← Override / Additive
├── ease_in: EaseType                     ← 開始時の補間形状（デフォルト: Linear）
├── ease_out: EaseType                    ← 終了時の補間形状（デフォルト: Linear）
├── muted: bool
└── cycle_count: f32                      ← ループ回数（1.0 = 1回、2.5 = 2.5回）
```

### Source と Instance の関係

```
ClipLibrary (Resource)
├── source_clips: HashMap<SourceClipId, SourceClip>
│     ├── "walk_cycle" (id=0)
│     ├── "idle"       (id=1)
│     └── "run"        (id=2)

Entity A
├── Animator { time: 0.5, speed: 1.0, playing: true, looping: true }
├── ClipSchedule
│     ├── instances:
│     │     ├── Instance{source=0, start=0.0, blend=Override} ← walk を 0秒〜
│     │     └── Instance{source=1, start=1.0, blend=Override} ← idle を 1秒〜

Entity B
├── Animator { time: 0.0, speed: 1.0, playing: true, looping: true }
├── ClipSchedule
│     └── instances:
│           └── Instance{source=0, start=0.0, blend=Override} ← 同じ walk をトリミング
```

Source の EditableAnimationClip を編集すると、全 Instance に反映される。
Instance 側はタイミング・トリミング・速度のみ制御。カーブデータは持たない。

### 既存構造との対応

| 既存 | 新設計 |
|------|--------|
| EditableAnimationClip | SourceClip.editable_clip |
| EditableClipId | SourceClipId |
| ClipBinding（設計案） | ClipInstance |
| editable_to_anim_id | SourceClip 内で管理 |

### ClipLibrary の拡張

```rust
pub struct ClipLibrary {
    pub animation: AnimationSystem,
    pub morph_animation: MorphAnimationSystem,
    source_clips: HashMap<SourceClipId, SourceClip>,
    dirty_sources: HashSet<SourceClipId>,
    next_source_id: SourceClipId,
    source_to_anim_id: HashMap<SourceClipId, AnimationClipId>,
}
```

既存の `editable_clips` を `source_clips` に統合。
内部的に EditableAnimationClip を保持する点は変わらない。

---

## 2. ブレンドシステム

### Maya の Animation Layers を採用しない理由

Maya の Animation Layers は「グローバルな処理パイプライン」の概念。
全属性に対してレイヤーを上から順番にブレンドする。この設計には以下の問題がある:

1. **layer_index 外部キーパターン**: ClipInstance が layer_index で AnimationLayer を参照する。
   2つの並列コレクションのインデックス相互参照は脆く、並べ替え/削除で壊れる。
2. **処理フローのデータ化**: レイヤーは「どう合成するか」という評価ロジックの設定であり、
   Entity のデータとしては不自然。ECS では system 関数側の責務。
3. **active_layer は UI 状態**: 「ユーザーが編集中のレイヤー」は表示状態であり、
   Entity コンポーネントではなく TimelineState（Resource）に属すべき。

### ECS 準拠の設計: per-instance BlendMode

ブレンドモードを AnimationLayer ではなく ClipInstance 自体に持たせる。

```
BlendMode
├── Override    ← 先行クリップを上書き（weight でブレンド）
└── Additive   ← 先行クリップに加算
```

各 ClipInstance が自身の blend_mode と weight を持つため、
レイヤーという中間概念が不要になる。

### ClipSchedule コンポーネント

```rust
pub struct ClipSchedule {
    pub instances: Vec<ClipInstance>,
}
```

Animator は変更なし（time, speed, playing, looping のみ）。
ClipSchedule はクリップ配置のみ。AnimationLayer 構造体は存在しない。

### 評価フロー

```
evaluate_entity_clips:
  1. instances を start_time 順にソート
  2. 現在時刻でアクティブな instances を抽出（muted はスキップ）
  3. 先頭の Override クリップ → base_pose
  4. 後続のクリップを順に適用:
     Override → blend_poses_override(current, new, weight)
     Additive → blend_poses_additive(current, new, rest, weight)
  5. 時間的に重なるクリップ → 自動クロスフェード（セクション3参照）
```

### ポーズブレンド用 system 関数

```rust
fn blend_poses_override(
    base: &SkeletonPose, overlay: &SkeletonPose, weight: f32,
    out: &mut SkeletonPose,
)

fn blend_poses_additive(
    base: &SkeletonPose, additive: &SkeletonPose, rest: &SkeletonPose,
    weight: f32, out: &mut SkeletonPose,
)
```

BoneLocalPose 単位で:
- Override: `lerp(base.translation, overlay.translation, weight)` + slerp for rotation
- Additive: `base.translation + (additive.translation - rest.translation) * weight`

### Maya の「レイヤーで一括 mute/weight」は？

Maya ではレイヤー単位で mute や weight を一括制御できるが、
これは **UI のグルーピング** 機能であり、データモデルではない。
TimelineState（Resource）で管理する:

```
TimelineState
├── clip_groups: HashMap<String, Vec<ClipInstanceId>>  ← UI用グルーピング
```

グループ内の全 Instance の muted/weight を UI 操作で一括変更できるが、
データとしては各 Instance が個別に muted/weight を保持する。

### 段階的実装

- Phase 1: 単一クリップ再生（現行と同等）
- Phase 2: 複数 Override クリップの時間配置 + クロスフェード
- Phase 3: Additive ブレンド対応
- Phase 4: Weight のアニメーション（時間変化する weight）

---

## 3. クリップブレンド/クロスフェード

Override クリップ同士が時間的に重なった場合の処理。
**追加のデータ構造は不要。system 関数のみで実現する。**

### 設計判断: TransitionCurve を採用しない理由

重なり領域は既存の ClipInstance データから完全に算出できる:
- overlap_start = max(clip_a.start_time, clip_b.start_time)
- overlap_end = min(clip_a.effective_end(), clip_b.effective_end())
- ブレンド重み = 区間内の正規化時間

TransitionCurve は「2つの ClipInstance 間の一時的な関係」であり:
- Entity のデータではない
- グローバル状態でもない
- 永続的に保持する必要がない

したがってクロスフェードは **system 関数の計算ロジック** として実装する。

### ブレンド領域

```
Track: ──[  Clip A  ]────
              [  Clip B  ]──
              ↑          ↑
           overlap_start  overlap_end
```

overlap 区間では system 関数が両クリップのポーズを重み付き合成する。

### system 関数

```rust
fn compute_crossfade_weight(
    clip_a: &ClipInstance,
    clip_b: &ClipInstance,
    current_time: f32,
) -> f32 {
    let overlap_start = clip_b.start_time;
    let overlap_end = clip_a.effective_end();
    let duration = overlap_end - overlap_start;
    if duration <= 0.0 { return 0.0; }

    let t = ((current_time - overlap_start) / duration).clamp(0.0, 1.0);
    apply_ease(t, clip_a.ease_out, clip_b.ease_in)
}

fn apply_ease(t: f32, ease_out: EaseType, ease_in: EaseType) -> f32 {
    // Linear: t をそのまま返す
    // EaseInOut: smoothstep(t) = t*t*(3-2*t)
    // 他のタイプも数学関数のみ。データ不要
}
```

入力は ClipInstance の既存フィールドのみ。中間データ構造は生成しない。

### ClipInstance への最小限の追加（非線形トランジション用）

```
ClipInstance
├── ...（既存フィールド）
├── ease_in: EaseType       ← このクリップが始まるときの補間形状
└── ease_out: EaseType      ← このクリップが終わるときの補間形状

EaseType
├── Linear       ← デフォルト。t をそのまま使用
├── EaseIn       ← 緩やかに開始
├── EaseOut      ← 緩やかに終了
├── EaseInOut    ← S カーブ
└── Stepped      ← 即時切替
```

これはクリップ自体の属性であり、ClipInstance に配置するのが自然。
初期実装では Linear のみで十分。EaseType は将来の拡張用。

### UI 表現

```
┌──────────────────────────────────────────────┐
│ ███ walk_cycle ████╲╱████ idle ██████████████ │
│                    ↑ クロスフェード領域        │
│                    （斜線で描画）              │
└──────────────────────────────────────────────┘
```

クロスフェード領域はクリップの端をドラッグして調整可能
（= ClipInstance の start_time / clip_out を変更するだけ）。

---

## 4. タンジェント/補間システム

### ECS 設計方針

EditableKeyframe は Resource（ClipLibrary）内部のデータ。
Component ではないが、Vec でイテレーションされるため 64 bytes（キャッシュライン）に収めるべき。

Maya の animCurve 体系には InterpolationType と TangentType の2つの概念があるが、
TangentType は「どう自動計算するか」という操作であり、ECS ではデータではなく system 関数の責務。

### 採用しない Maya の要素と理由

| Maya の要素 | 不採用理由 |
|-------------|-----------|
| TangentType (Auto/Spline/Flat/...) | 操作であり、データではない。system 関数に移行 |
| tangent_broken: bool | in/out タンジェントの値から導出可能 |
| weighted_tangent: bool | weight がデフォルトかどうかで判定可能 |
| Hermite 補間 | Bezier で表現可能。重複排除 |
| Constant 補間 | Stepped と同義。重複排除 |

### InterpolationType（3種のみ）

```
InterpolationType
├── Linear    ← 直線補間。タンジェントハンドル不使用
├── Bezier    ← 3次ベジェ。BezierHandle で制御
└── Stepped   ← 次のキーまで値保持
```

### EditableKeyframe（既存からの最小変更）

```rust
pub struct EditableKeyframe {    // ~36 bytes → キャッシュライン内
    pub id: KeyframeId,              // 8 bytes
    pub time: f32,                   // 4 bytes
    pub value: f32,                  // 4 bytes
    pub interpolation: InterpolationType,  // 4 bytes (enum + padding)
    pub in_tangent: BezierHandle,    // 8 bytes（既存構造体そのまま）
    pub out_tangent: BezierHandle,   // 8 bytes（既存構造体そのまま）
}

pub struct BezierHandle {            // 既存のまま変更なし
    pub time_offset: f32,
    pub value_offset: f32,
}
```

現行の EditableKeyframe (~32 bytes) に InterpolationType (4 bytes) を追加するだけ。
BezierHandle は既存構造をそのまま使用。新しい構造体は導入しない。

### タンジェント操作 = system 関数

Maya の TangentType に相当する操作は、BezierHandle の値を計算して書き込む system 関数:

```rust
fn apply_auto_tangent(curve: &mut PropertyCurve, keyframe_id: KeyframeId)
fn apply_flat_tangent(curve: &mut PropertyCurve, keyframe_id: KeyframeId)
fn apply_linear_tangent(curve: &mut PropertyCurve, keyframe_id: KeyframeId)
fn apply_spline_tangent(curve: &mut PropertyCurve, keyframe_id: KeyframeId)
```

例: `apply_auto_tangent` は前後キーの値を参照し、クランプ付きで BezierHandle を計算する。
保存されるのは計算結果（time_offset, value_offset）のみ。TangentType 自体は保存しない。

### ベジェ補間の計算（system 関数）

```rust
fn sample_bezier(k0: &EditableKeyframe, k1: &EditableKeyframe, t: f32) -> f32 {
    let dt = k1.time - k0.time;
    let p0 = (k0.time, k0.value);
    let p1 = (k0.time + k0.out_tangent.time_offset, k0.value + k0.out_tangent.value_offset);
    let p2 = (k1.time + k1.in_tangent.time_offset, k1.value + k1.in_tangent.value_offset);
    let p3 = (k1.time, k1.value);
    // De Casteljau アルゴリズムで評価
}
```

### PropertyCurve::sample() の拡張

```rust
fn sample(&self, time: f32) -> Option<f32> {
    // ... 既存の範囲チェック ...
    let t = (time - k0.time) / (k1.time - k0.time);
    match k0.interpolation {
        Stepped => Some(k0.value),
        Linear => Some(k0.value + (k1.value - k0.value) * t),
        Bezier => Some(sample_bezier(k0, k1, t)),
    }
}
```

k0.interpolation のみで分岐。k1 は参照しない。
sample() は self のみを読む純粋なアクセサであり、メソッドのまま許容する。
将来的に計算が複雑化した場合に system 関数へ移行する。

### サイズ影響分析

典型的なアニメーションクリップ: 24 bones × 6 curves × 30 keyframes = 4,320 keyframes

| 設計 | keyframe サイズ | クリップ合計 |
|------|----------------|-------------|
| 現行 | ~32 bytes | ~138 KB |
| 本設計（+InterpolationType） | ~36 bytes | ~155 KB |
| Maya 全部入り（不採用） | ~48 bytes | ~207 KB |

いずれもデスクトップアプリとして問題ないサイズ。

---

## 5. Dope Sheet モード

タイムラインウィンドウ内の表示モード。キーフレームのタイミング概観に特化。

### UI レイアウト
**重要** : 基本的にはMayaを参考にするが、データのECS設計を第一とする。
そのため、Mayaとの乖離は認める。

```
┌─ Dope Sheet ──────────────────────────────────────────┐
│         │  0.0   0.5   1.0   1.5   2.0   2.5   3.0   │
│─────────┼─────────────────────────────────────────────│
│ ▶ Summary   │ ■  ■ ■    ■  ■     ■  ■ ■    ■        │ ← 全キー集約
│─────────┼─────────────────────────────────────────────│
│ ▼ Hips      │ ◆  ◆      ◆        ◆  ◆      ◆        │ ← ボーン集約
│   Trans X   │ ◇  ◇      ◇        ◇  ◇      ◇        │ ← 個別プロパティ
│   Trans Y   │ ◇         ◇              ◇    ◇        │
│   Trans Z   │ ◇  ◇      ◇        ◇  ◇      ◇        │
│   Rot X     │ ◆  ◆      ◆        ◆  ◆      ◆        │
│   ...       │                                         │
│─────────┼─────────────────────────────────────────────│
│ ▶ Spine     │ ◆     ◆   ◆   ◆        ◆   ◆          │
│ ▶ L_Arm     │ ◆  ◆  ◆   ◆   ◆   ◆  ◆   ◆           │
└─────────────────────────────────────────────────────────┘
```

### Summary Row

ボーン行を折りたたみ時: 配下の全プロパティカーブのキーフレーム時刻を集約表示。
選択 → 配下全キーフレームが選択。
移動 → 配下全キーフレームが同量移動。

### キーフレーム表現

| 記号 | 意味 |
|------|------|
| ◆ | 通常キーフレーム（ボーン集約行） |
| ◇ | 通常キーフレーム（個別プロパティ行） |
| ■ | Summary 行キーフレーム |
| ◆（緑） | Breakdown キー |
| ◆（赤） | 選択中キーフレーム |

### 操作

- **ボックス選択**: ドラッグで矩形選択
- **Shift+クリック**: 選択追加
- **Ctrl+クリック**: 選択トグル
- **ドラッグ移動**: 選択キーフレーム群を時間方向に移動
- **Scale 操作**: 選択範囲の左端/右端をドラッグで時間スケール
- **右クリックメニュー**: Insert Key / Delete Keys / Copy / Paste / Mirror

---

## 6. Graph Editor マルチクリップ統合

### 現状の課題

現在の CurveEditorState は TimelineState.current_clip_id の単一クリップのみ表示。
マルチクリップ対応では、active_clip 以外のクリップカーブもオーバーレイ表示できる必要がある。

### ECS 設計方針: God Resource の回避

CurveEditorState に全状態を詰め込むと God Resource になる。
責務ごとに分離する:

- **CurveEditorState (Resource)**: 編集対象の指定のみ
- **CurveEditorViewSettings (Resource)**: 表示設定（永続）
- **CurveEditorBuffer (Resource)**: バッファカーブのスナップショット

```
CurveEditorState (Resource — 編集対象)
├── primary_source_id: Option<SourceClipId>
├── selected_bone_id: Option<BoneId>
├── ... (既存の選択・フォーカス状態)

CurveEditorViewSettings (Resource — 表示設定)
├── overlay_source_ids: Vec<SourceClipId>
├── overlay_opacity: f32
├── visible_curves: HashSet<PropertyType>
├── normalized_view: bool
├── show_buffer_curve: bool

CurveEditorBuffer (Resource — 比較用スナップショット)
├── snapshots: HashMap<(BoneId, PropertyType), Vec<(f32, f32)>>
```

バッファカーブは SourceClip のコピーではなく、明示的な「比較用スナップショット」。
SourceClip が正本であることは変わらない。

### バッファカーブ（Maya の Buffer Curves に対応）

編集前のカーブスナップショットを保持し、比較表示する機能。
カーブ編集開始時に自動的に現在のカーブをバッファに保存。
「Swap Buffer」で元に戻す操作も可能。

```rust
fn capture_buffer_curve(
    buffer: &mut CurveEditorBuffer,
    clip: &EditableAnimationClip,
    bone_id: BoneId,
    property: PropertyType,
)

fn swap_buffer_curve(
    buffer: &mut CurveEditorBuffer,
    clip: &mut EditableAnimationClip,
    bone_id: BoneId,
    property: PropertyType,
)
```

### 正規化表示

全カーブを -1.0〜1.0 の範囲に正規化して同一スケールで表示。
Translation（cm 単位）と Rotation（度単位）のような異なるスケールのカーブを
重ねて比較できる。

```
original_range = curve_max - curve_min
normalized_value = (value - curve_min) / original_range * 2.0 - 1.0
```

正規化は描画時に計算する。正規化後の値は保存しない。

### オーバーレイ表示

別クリップのカーブを薄い色で重ねて表示。
ウォークサイクルを参照しながらランサイクルを編集、といったワークフローを支援。
overlay_source_ids は CurveEditorViewSettings が保持。カーブデータ自体は ClipLibrary から読み取る。

---

## 7. Per-Entity GPU リソース管理

マルチモデル対応のための GPU バッファ設計。

### 現状の課題

現在の GraphicsResources はグローバルに1つ。
複数モデルが同時にアニメーションする場合、各モデルが独自の頂点バッファを持つ必要がある。

### ECS 設計方針: Component vs Resource HashMap

Entity ごとのデータを `HashMap<Entity, Data>` で Resource に入れるのは
手動 Entity マッピングであり、ECS の Component パターンではない。

ただし GPU バッファ（RRVertexBuffer 等）は Vulkan リソースのライフサイクル管理が必要であり、
Component として Entity に直接持たせると所有権の移動やドロップ順序の問題が生じる。

判断: **GPU バッファは Resource の HashMap で管理し、メタデータは Component に持たせる。**
skeleton_id, animation_type 等は ModelState Component が正本。EntityRenderData には重複して持たない。

### RenderData（Entity ごとの GPU バッファ）

```
RenderData
├── mesh_buffers: Vec<MeshBuffer>
├── skin_data: Option<SkinData>
├── node_data: Vec<NodeData>
└── dirty: bool

MeshBuffer
├── vertex_buffer: RRVertexBuffer
├── index_buffer: RRIndexBuffer
├── base_vertices: Vec<Vertex>
├── local_vertices: Vec<Vertex>
├── descriptor_set: RRDescriptorSet
└── material_index: usize
```

skeleton_id, animation_type, node_animation_scale は含まない。
これらは ModelState Component から読み取る。

### GraphicsResources の拡張

```rust
pub struct GraphicsResources {
    pub render_data: HashMap<Entity, RenderData>,
    pub shared_textures: Vec<RRImage>,
    pub shared_materials: Vec<Material>,
    pub pipelines: Vec<RRPipeline>,
}
```

### ModelState (Component)

既に Entity コンポーネントとして存在。GPU リソース側には重複しない。

```rust
pub struct ModelState {
    pub has_skinned_meshes: bool,
    pub node_animation_scale: f32,
    pub model_path: String,
    pub animation_type: AnimationType,
}
```

### 描画フロー（変更後）

```
render_all_entities:
  for (entity, render_data) in graphics.render_data.iter():
    let model_state = world.get_component::<ModelState>(entity);
    if render_data.dirty:
      update_vertex_buffers(render_data, model_state)
    record_draw_commands(render_data)
```

ModelState は正本として Component から読み取る。RenderData はそれを参照するのみ。

---

## 8. Undo/Redo システム

アニメーション編集では操作の取り消し/やり直しが必須。

### ECS 設計方針: God Context の回避

GoF の Command パターンでは `execute(context)` に全リソースを渡すのが一般的だが、
これは God Context（なんでもできるコンテキスト）を生む。

ECS では各コマンドが**操作対象を明示的に宣言**すべき。
コマンドの種類に応じて、必要なリソースのみを渡す:

- キーフレーム操作 → `&mut ClipLibrary` のみ
- インスタンス操作 → `&mut World`（ClipSchedule Component へのアクセス）のみ

### コマンド設計

```
EditCommand (enum — trait ではなく enum)
├── MoveKeyframe { ... }
├── AddKeyframe { ... }
├── DeleteKeyframes { ... }
├── ModifyTangent { ... }
├── MoveClipInstance { ... }
├── TrimClipInstance { ... }
├── ChangeInstanceBlendMode { ... }
├── ChangeInstanceWeight { ... }

EditHistory (Resource)
├── undo_stack: Vec<EditEntry>
├── redo_stack: Vec<EditEntry>
└── max_history: usize

EditEntry
├── command: EditCommand
└── group_id: Option<u64>
```

trait ではなく enum を採用する理由:
- コマンドの種類は有限で列挙可能
- シリアライズ/デシリアライズが容易（将来のファイル保存）
- match で操作対象を分岐し、必要なリソースのみ渡せる

### コマンド実行の分岐

```rust
fn execute_command(cmd: &EditCommand, clip_library: &mut ClipLibrary, world: &mut World) {
    match cmd {
        MoveKeyframe { source_id, .. } => {
            // clip_library のみ使用
        }
        MoveClipInstance { entity, .. } => {
            // world のみ使用（ClipSchedule Component）
        }
    }
}

fn undo_command(cmd: &EditCommand, clip_library: &mut ClipLibrary, world: &mut World) {
    // 同様に match で分岐
}
```

description はコマンドに持たせず、UI 側で match して表示文字列を生成する。

### 主要コマンド

```
MoveKeyframe
├── source_id: SourceClipId
├── bone_id: BoneId
├── property: PropertyType
├── keyframe_id: KeyframeId
├── old_time: f32, old_value: f32
└── new_time: f32, new_value: f32

AddKeyframe
├── source_id, bone_id, property
├── keyframe: EditableKeyframe
└── created_id: Option<KeyframeId>

DeleteKeyframes
├── deleted: Vec<(SourceClipId, BoneId, PropertyType, EditableKeyframe)>

ModifyTangent
├── source_id, bone_id, property, keyframe_id
├── old_in/out_tangent, new_in/out_tangent

MoveClipInstance
├── entity: Entity
├── instance_id: ClipInstanceId
├── old_start_time: f32
└── new_start_time: f32

TrimClipInstance
├── entity, instance_id
├── old_clip_in/out, new_clip_in/out

ChangeInstanceBlendMode
├── entity, instance_id
├── old_blend_mode, new_blend_mode

ChangeInstanceWeight
├── entity, instance_id
├── old_weight, new_weight
```

### グループ化

連続ドラッグ操作を1つの Undo 単位にグループ化。
同じ group_id を持つ EditEntry が連続する場合、Undo 時に一括で取り消す。

```
push_command(history, cmd, group_id: Option<u64>)
undo(history, clip_library, world)  // group_id が同じ連続エントリを一括 undo
redo(history, clip_library, world)
```

---

## 9. クリップ管理 UI

### Clip Browser（新規ウィンドウ）

```
┌─ Clip Browser ──────────────────────────┐
│ [+ New] [Import] [Duplicate]            │
│─────────────────────────────────────────│
│ 🔍 Filter: [____________]               │
│─────────────────────────────────────────│
│ > walk_cycle      1.20s  2 refs         │
│ > idle            3.00s  1 ref          │
│ > run             0.80s  0 refs         │
│ > attack_slash    1.50s  1 ref          │
│─────────────────────────────────────────│
│ Selected: walk_cycle                    │
│ Duration: 1.20s                         │
│ Bones: 24                               │
│ Source: assets/models/stickman.glb      │
└─────────────────────────────────────────┘
```

### 操作

- **New**: 空の SourceClip を作成
- **Import**: 別の glTF/FBX からクリップをインポート
- **Duplicate**: 選択中の SourceClip をコピー（独立した新 Source）
- **Rename**: ダブルクリックで名前変更
- **Delete**: 参照カウント 0 の Source のみ削除可能
- **D&D to Timeline**: Clip Browser から Timeline Track へドラッグで ClipInstance 作成

---

## 10. 選択モデルとキーボードショートカット

### ECS 設計方針: 選択状態の重複回避

既存の選択状態:
- `HierarchyState.selected_entity` — シーン階層での Entity 選択
- `TimelineState.selected_keyframes` — タイムラインでのキーフレーム選択

新しい SelectionState を追加すると selected_entities / selected_keyframes が
既存と重複し、整合性維持のコストが発生する。

判断: **新規 Resource は作らない。既存 Resource を拡張する。**

- Entity 選択 → `HierarchyState.selected_entity`（既存、変更なし）
- キーフレーム選択 → `TimelineState.selected_keyframes`（既存、変更なし）
- ClipInstance 選択 → `TimelineState.selected_instances: HashSet<ClipInstanceId>`（追加）

### SelectionMode は永続状態ではない

Replace / Add / Toggle / BoxSelect は「どう選択するか」の操作方法であり、
ユーザーの修飾キー（Shift/Ctrl）とマウス操作から決定される。
永続的な状態として保存する必要がない。

system 関数のパラメータとして渡す:

```rust
fn select_keyframe(
    timeline_state: &mut TimelineState,
    keyframe: SelectedKeyframe,
    modifier: SelectionModifier,
)

SelectionModifier
├── Replace    ← 修飾キーなし
├── Add        ← Shift
└── Toggle     ← Ctrl
```

BoxSelect はドラッグ操作中の一時状態であり、UI コード内で処理する。

### キーボードショートカット

**トランスポート**:

| キー | 操作 |
|------|------|
| Space | 再生/一時停止 |
| , (カンマ) | 前のフレーム |
| . (ピリオド) | 次のフレーム |
| Home | 先頭へ |
| End | 末尾へ |
| Alt+, | 前のキーフレームへ |
| Alt+. | 次のキーフレームへ |

**編集**:

| キー | 操作 |
|------|------|
| S | 現在時刻にキーフレーム挿入 |
| Delete | 選択キーフレーム削除 |
| Ctrl+C | キーフレームコピー |
| Ctrl+V | キーフレームペースト |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+A | 全選択 |
| F | 選択範囲にフィット（ズーム） |
| A | 全体表示にフィット |

**Graph Editor**:

| キー | 操作 |
|------|------|
| 1 | Auto タンジェント適用（apply_auto_tangent） |
| 2 | Spline タンジェント適用（apply_spline_tangent） |
| 3 | Linear 補間に変更 |
| 4 | Flat タンジェント適用（apply_flat_tangent） |
| 5 | Stepped 補間に変更 |
| B | バッファカーブ表示切替 |
| N | 正規化表示切替 |

**Timeline**:

| キー | 操作 |
|------|------|
| L | ループ切替 |
| M | 選択トラック Mute 切替 |
| Tab | Dope Sheet ↔ Curve Editor 切替 |

---

## 11. コピー/ペースト/ミラー

### コピーバッファ

```
KeyframeCopyBuffer
├── entries: Vec<CopiedKeyframe>
├── base_time: f32                  ← コピー時の先頭キーフレーム時刻
└── source_clip_id: SourceClipId

CopiedKeyframe
├── bone_id: BoneId
├── property: PropertyType
├── relative_time: f32              ← base_time からの相対時刻
├── value: f32
├── interpolation: InterpolationType
├── in_tangent: BezierHandle
└── out_tangent: BezierHandle
```

### ペースト

ペースト時、プレイヘッド位置を基準に relative_time を加算して配置。
同一 bone_id + property にペースト。

### ミラー

左右対称ボーンのアニメーションを反転コピー。

MirrorMapping はデータ構造。処理はすべて system 関数。

```
MirrorMapping
├── pairs: Vec<(BoneId, BoneId)>      ← (L_Arm, R_Arm) のペア
└── symmetry_axis: Axis               ← X / Y / Z
```

### system 関数

```rust
fn build_mirror_mapping(skeleton: &Skeleton) -> MirrorMapping
```

ボーン名パターン ("L_" ↔ "R_" / "Left" ↔ "Right" / "_l" ↔ "_r") から自動推定。

```rust
fn mirror_keyframes(
    buffer: &KeyframeCopyBuffer,
    mapping: &MirrorMapping,
) -> KeyframeCopyBuffer
```

コピーバッファの各キーフレームについて:
1. bone_id → ミラーペアの相手に変換
2. TranslationX → 符号反転（対称軸に応じて）
3. RotationY, RotationZ → 符号反転
4. 新しいコピーバッファとして返す（ペースト操作は呼び出し元が行う）

---

## 12. スナッピング

### スナップモード

```
SnapSettings
├── snap_to_frame: bool         ← フレーム単位にスナップ
├── snap_to_key: bool           ← 近傍キーフレームにスナップ
├── frame_rate: f32             ← フレームレート（24, 30, 60 fps 等）
├── snap_threshold: f32         ← スナップ判定距離（ピクセル）
└── snap_to_grid: bool          ← Graph Editor のグリッドスナップ
```

### スナップ処理

```
snap_time(raw_time: f32, settings: &SnapSettings) -> f32:
  if snap_to_frame:
    return (raw_time * frame_rate).round() / frame_rate
  if snap_to_key:
    nearest = find_nearest_keyframe_time(raw_time)
    if |nearest - raw_time| < threshold:
      return nearest
  return raw_time
```

---

## 13. パフォーマンス考慮

### マルチ Entity 評価

| 項目 | 対策 |
|------|------|
| 多数 Entity のポーズ計算 | Entity ごとに独立 → 将来的に並列化可能 |
| Skeleton キャッシュ | AssetStorage からの取得は HashMap 1回。SkeletonPose は毎フレーム再生成（軽量） |
| ClipInstance 評価 | active clip のみ評価。muted/weight=0 はスキップ |
| ポーズブレンド | アクティブ ClipInstance 数は通常 2-3。O(instances * bones) で軽量 |
| dirty 追跡 | SourceClip の dirty フラグで変更時のみ AnimationClip を再構築 |

### GPU バッファ管理

```
Entity 追加時:
  → EntityRenderData 生成、バッファ確保

Entity 削除時:
  → EntityRenderData 削除、バッファ解放

アニメーション評価後:
  → dirty な Entity の頂点バッファのみ更新
```

### 遅延評価

```
evaluate_animators 内:
  if animator.time == previous_time && !clip_dirty:
    skip evaluation (前フレームと同じ)
```

停止中の Entity は頂点バッファ更新をスキップ。

---

## 14. 評価パイプライン詳細

### evaluate_entity_animation（単一 Entity の完全評価）

```
fn evaluate_entity_animation(
    animator: &Animator,
    schedule: &ClipSchedule,
    skeleton: &Skeleton,
    clip_library: &ClipLibrary,
) -> SkeletonPose {

    let active_instances: Vec<&ClipInstance> = schedule.instances.iter()
        .filter(|c| !c.muted && is_clip_active(c, animator.time))
        .collect();

    if active_instances.is_empty() {
        return create_pose_from_rest(skeleton);
    }

    let mut current_pose = create_pose_from_rest(skeleton);

    for instance in &active_instances {
        let local_time = compute_local_time(instance, animator.time);
        let source = clip_library.get_source(instance.source_id);
        let anim_clip = source.editable_clip.to_animation_clip();

        let mut instance_pose = create_pose_from_rest(skeleton);
        sample_clip_to_pose(&anim_clip, local_time, skeleton, &mut instance_pose, ...);

        match instance.blend_mode {
            Override => {
                let weight = compute_effective_weight(instance, &active_instances, animator.time);
                blend_poses_override(&current_pose, &instance_pose, weight, &mut current_pose);
            }
            Additive => {
                let rest = create_pose_from_rest(skeleton);
                blend_poses_additive(&current_pose, &instance_pose, &rest, instance.weight, &mut current_pose);
            }
        }
    }

    current_pose
}
```

### compute_effective_weight（クロスフェード考慮の重み計算）

ClipInstance の既存フィールドのみを入力とする純粋な system 関数。
中間データ構造は生成しない（セクション3参照）。

```
fn compute_effective_weight(
    instance: &ClipInstance,
    all_active: &[&ClipInstance],
    current_time: f32,
) -> f32 {
    let overlapping = find_override_overlap(instance, all_active, current_time);
    match overlapping {
        None => instance.weight,
        Some(other) => {
            let factor = compute_crossfade_weight(instance, other, current_time);
            instance.weight * factor
        }
    }
}
```

### ローカル時間の計算

```
fn compute_local_time(instance: &ClipInstance, global_time: f32) -> f32 {
    let elapsed = (global_time - instance.start_time) * instance.speed;
    let clip_duration = instance.clip_out - instance.clip_in;

    if instance.cycle_count > 0.0 {
        let total = clip_duration * instance.cycle_count;
        let clamped = elapsed.min(total);
        instance.clip_in + (clamped % clip_duration)
    } else {
        instance.clip_in + elapsed.clamp(0.0, clip_duration)
    }
}
```

---

## 15. Timeline Window UI 詳細レイアウト（改訂）

Phase B 以降の完成形レイアウト。

```
┌─ Timeline ─────────────────────────────────────────────────────────────────────┐
│ [▶] [⏸] [⏹]  [⟳]Loop  Speed:[1.0x▼]  0:01.25 / 0:03.00  [−][+] Snap:[F▼]  │
│ [Dope Sheet] [Graph Editor]  ← 表示モード切替タブ                              │
│────────────────┬───────────────────────────────────────────────────────────────│
│                │  0:00   0:00.5  0:01   0:01.5  0:02   0:02.5  0:03          │
│                │    |      |      |   ▼   |      |      |      |             │
│────────────────┼───────────────────────────────────────────────────────────────│
│                │                                                              │
│ ▼ Stickman     │ ████ walk_cycle ██████╲╱████ idle ████████████████           │
│  [M][S][L]     │         ↑ ClipInstance 矩形ブロック                          │
│                │    (Override 同士の重なり → 自動クロスフェード)               │
│                │                                                              │
│  ▼ walk_cycle  │                                                              │
│   ▶ Summary   │ ■  ■ ■    ■  ■     ■  ■ ■    ■                              │
│   ▼ Hips      │ ◆  ◆      ◆        ◆  ◆      ◆                              │
│     Trans X   │ ◇  ◇      ◇        ◇  ◇      ◇                              │
│     Trans Y   │ ◇         ◇              ◇    ◇                              │
│     Rot X     │ ◆  ◆      ◆        ◆  ◆      ◆                              │
│   ▶ Spine     │ ◆     ◆   ◆   ◆        ◆   ◆                                │
│   ▶ L_Arm     │ ◆  ◆  ◆   ◆   ◆   ◆  ◆   ◆                                 │
│                │                                                              │
│────────────────┼───────────────────────────────────────────────────────────────│
│                │                                                              │
│ ▼ Dragon      │ ████████ fly_loop ████████████████████████████                │
│  [M][S][L]    │                                                              │
│                │                                                              │
│ ▶ Camera      │ (no animation)                                               │
│                │                                                              │
└────────────────┴───────────────────────────────────────────────────────────────┘
```

### 階層構造（改訂）

```
Level 0: Entity Track             ← Animator + ClipSchedule を持つ Entity
  Level 1: ClipInstance Block     ← ClipSchedule.instances の各クリップ配置
  Level 2: Clip Detail            ← 選択中クリップの展開
    Level 2.5: Summary Row        ← 全ボーンキーフレーム集約
    Level 3: Bone Track           ← ボーン単位
      Level 4: Property Curve     ← TransX/TransY/... 個別
```

### 右クリックコンテキストメニュー

**Entity Track 上**:
- Bind Clip from Library...
- Group Selected Clips...

**ClipInstance 上**:
- Blend Mode → Override / Additive
- Trim Start / Trim End
- Set Speed...
- Cycle Count...
- Duplicate Instance
- Remove Instance

**Keyframe 上**:
- Set Tangent → Auto / Spline / Linear / Flat / Stepped
- Break Tangents
- Insert Key at Playhead
- Delete Selected Keys
- Copy Keys / Paste Keys / Mirror Keys

---

## 16. イベント拡張（改訂）

設計案の UIEvent に加えて、新機能対応のイベント:

```
// Source/Instance 管理
ClipBrowserCreateSource { name: String }
ClipBrowserDuplicateSource(SourceClipId)
ClipBrowserDeleteSource(SourceClipId)
ClipBrowserRenameSource { id: SourceClipId, name: String }

// Instance 操作
TimelineCreateInstance { entity: Entity, source_id: SourceClipId, start_time: f32 }
TimelineRemoveInstance { entity: Entity, instance_id: ClipInstanceId }
TimelineMoveInstance { entity: Entity, instance_id: ClipInstanceId, new_start: f32 }
TimelineTrimInstance { entity: Entity, instance_id: ClipInstanceId, clip_in: f32, clip_out: f32 }
TimelineSetInstanceSpeed { entity: Entity, instance_id: ClipInstanceId, speed: f32 }
TimelineSetInstanceCycleCount { entity: Entity, instance_id: ClipInstanceId, count: f32 }

// Instance ブレンド操作
TimelineSetInstanceBlendMode { entity: Entity, instance_id: ClipInstanceId, mode: BlendMode }
TimelineSetInstanceWeight { entity: Entity, instance_id: ClipInstanceId, weight: f32 }

// グルーピング（UI 用）
TimelineGroupClips { entity: Entity, group_name: String, instance_ids: Vec<ClipInstanceId> }
TimelineUngroupClips { entity: Entity, group_name: String }
TimelineMuteGroup { entity: Entity, group_name: String }
TimelineUnmuteGroup { entity: Entity, group_name: String }

// タンジェント操作
// TangentPreset はイベントディスパッチ用の列挙。データには保存しない。
// イベントハンドラが対応する system 関数（apply_auto_tangent 等）を呼び出す。
// TangentPreset { Auto, Spline, Linear, Flat }
CurveEditorApplyTangentPreset { bone_id, property, keyframe_id, preset: TangentPreset }
CurveEditorBreakTangents { bone_id, property, keyframe_id }
CurveEditorSetInterpolation { bone_id, property, keyframe_id, interpolation: InterpolationType }

// Undo/Redo
EditUndo
EditRedo

// スナップ
TimelineSetSnapMode(SnapSettings)
```

---

## 17. 段階的実装計画（改訂）

上記設計案 Phase A-E を詳細化:

### Phase A: Source/Instance + ClipSchedule 導入
1. SourceClip / ClipInstance / BlendMode データ構造定義
2. ClipSchedule コンポーネント定義（instances のみ）
3. ClipLibrary を source_clips ベースに変換
4. 既存の Animator.current_clip_id を ClipSchedule.instances[0] に移行する互換レイヤー
5. evaluate_animators を Animator + ClipSchedule ベースに変更

### Phase B: マルチ Entity 評価
1. evaluate_all_animators 実装（全 Animated Entity をイテレート）
2. Per-Entity GPU リソース（RenderData）導入（メタデータは ModelState Component が正本）
3. ModelState の Entity コンポーネント化
4. AnimationPlayback への依存除去

### Phase C: Timeline マルチトラック UI
1. TimelineState 拡張（track_states, active_track）
2. Timeline Window の Entity Track 表示
3. ClipInstance の矩形ブロック描画
4. クリップのドラッグ移動/トリミング UI

### Phase D: ブレンドシステム
1. Override クリップ間のクロスフェード評価
2. Additive ブレンド評価（blend_poses_additive）
3. compute_effective_weight によるクロスフェード重み計算
4. UI グルーピング機能（TimelineState.clip_groups）
5. グループ一括 Mute/Weight UI

### Phase E: タンジェント/補間
1. InterpolationType enum 追加（Linear / Bezier / Stepped）
2. EditableKeyframe に interpolation フィールド追加
3. sample_bezier system 関数実装
4. PropertyCurve::sample() の InterpolationType 分岐対応
5. apply_auto/flat/linear/spline_tangent system 関数群
6. Graph Editor のタンジェントハンドル描画・ドラッグ UI

### Phase F: Dope Sheet + 詳細編集
1. Dope Sheet 表示モード実装
2. Summary Row
3. SelectionModifier による選択操作（既存 TimelineState を拡張）
4. コピー/ペースト/ミラー（build_mirror_mapping, mirror_keyframes system 関数）
5. スナッピング
6. バッファカーブ（CurveEditorBuffer Resource）

### Phase G: Undo/Redo + Clip Browser
1. EditCommand enum + EditHistory Resource
2. execute_command / undo_command system 関数（操作対象に応じたリソース分岐）
3. Clip Browser ウィンドウ
4. D&D によるクリップバインド

---

## 18. 既存コードとの対応表

| ファイル | 変更概要 |
|----------|---------|
| `ecs/world.rs` Animator | 変更なし（再生状態のみ維持） |
| (新規) `ecs/component/clip_schedule.rs` | ClipSchedule コンポーネント（instances のみ、layers なし） |
| `ecs/resource/timeline_state.rs` | track_states, active_track, active_clip 等追加 |
| `ecs/resource/clip_library.rs` | source_clips ベースに変換 |
| `ecs/resource/graphics.rs` ModelState | Entity コンポーネント化 |
| `animation/editable/keyframe.rs` | InterpolationType 追加（BezierHandle は変更なし） |
| `animation/editable/curve.rs` | sample() の InterpolationType 分岐 |
| `ecs/systems/animation_playback_systems.rs` | evaluate_all_animators（Animator + ClipSchedule 参照）、インスタンスブレンド評価 |
| `ecs/systems/skeleton_pose_systems.rs` | blend_poses_override, blend_poses_additive 追加 |
| `platform/ui/timeline_window.rs` | マルチトラック UI、Dope Sheet モード |
| `platform/ui/curve_editor_window.rs` | タンジェントハンドル、正規化表示 |
| `ecs/events/ui_events.rs` | 新規イベント群追加 |
| `app/graphics_resource.rs` | RenderData（GPU バッファのみ）、Per-Entity バッファ |
| `ecs/resource/timeline_state.rs` | selected_instances 追加（選択状態の拡張） |
| (新規) `ecs/resource/edit_history.rs` | EditHistory + EditCommand enum |
| (新規) `ecs/resource/curve_editor_view.rs` | CurveEditorViewSettings + CurveEditorBuffer |
| (新規) `platform/ui/clip_browser_window.rs` | Clip Browser |
