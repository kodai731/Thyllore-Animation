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

**核心**: Entity が Animator コンポーネントを持ち、Animator がどのクリップを再生するかを保持する。
Timeline Window は Animator を持つ Entity を自動的にトラックとして表示する。

## ECS データ設計

### Component（Entity に付くデータ）

```
Animator (既存を拡張)
├── clip_bindings: Vec<ClipBinding>    ← この Entity にバインドされた全クリップ
├── active_clip_index: Option<usize>   ← 現在再生中のバインディングインデックス
├── time: f32
├── speed: f32
├── playing: bool
└── looping: bool

ClipBinding
├── clip_id: EditableClipId            ← ClipLibrary 内のクリップ参照
├── start_time: f32                    ← タイムライン上の配置開始時刻
├── duration_override: Option<f32>     ← トリミング用（None = クリップのフル duration）
├── weight: f32                        ← ブレンド重み（将来のレイヤー対応用）
└── muted: bool                        ← ミュート状態
```

**Animated** (既存マーカー): 変更なし。Animator を持つ Entity に付与。

### Resource（グローバル状態）

```
ClipLibrary (既存を拡張)
├── animation: AnimationSystem         ← clips のみ（Skeleton は AssetStorage が正本）
├── morph_animation: MorphAnimationSystem
├── editable_clips: HashMap<EditableClipId, EditableAnimationClip>
├── dirty_clips, next_editable_id, editable_to_anim_id
└── (変更なし。クリップの所有はここ、バインディングは Animator 側)
```

ClipLibrary はクリップの「倉庫」のまま。Skeleton を含まない。
どの Entity がどのクリップを使うかは Animator.clip_bindings で表現。
= Maya の clipLibrary + clipScheduler 分離に対応。

```
AssetStorage (正本)
├── skeletons: HashMap<AssetId, SkeletonAsset>   ← Skeleton の唯一の所有者
└── get_skeleton_by_skeleton_id(SkeletonId)      ← SkeletonId で検索
```

```
TimelineState (大幅拡張)
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
├── active_clip_id: Option<EditableClipId>     ← 現在編集中のクリップ
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
  Level 1: Clip Block         ← clip_bindings の各 ClipBinding
    Level 2: Bone Track       ← EditableAnimationClip.tracks の各 BoneTrack
      Level 3: Property Curve ← BoneTrack 内の translation_x 等
```

## ECS Systems 設計

### 新規 System

```
evaluate_all_animators (animation_playback_systems.rs を拡張)
  全 Animated Entity をイテレートし、各 Animator の状態に基づいてアニメーション評価。
  現在の evaluate_animators を複数 Entity 対応に拡張。

  処理フロー:
    for (entity, animator) in world.iter_animated_entities():
      skeleton = assets.get_skeleton_by_skeleton_id(skel_id)
      clip = clip_library.animation.get_clip(clip_id)
      let mut pose = create_pose_from_rest(skeleton)
      sample_clip_to_pose(clip, local_time, skeleton, &mut pose, looping)
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
     （各 ClipBinding の start_time を考慮した local_time を計算）
  3. dirty クリップを sync
```

```
run_animation_phase_ecs 詳細:
  for each Animated Entity:
    1. Animator から time, current_clip_id, looping を取得
    2. assets.get_skeleton_by_skeleton_id(skel_id) で不変 Skeleton 取得
    3. create_pose_from_rest → sample_clip_to_pose → compute_pose_global_transforms
    4. AnimationType に応じて prepare_skinned_vertices / prepare_node_animation
```

## 段階的移行計画

### Phase A: Animator.clip_bindings 導入
- Animator に clip_bindings フィールド追加
- 既存の current_clip_id を clip_bindings[0] に変換する互換レイヤー
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
