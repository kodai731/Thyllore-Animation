# リリース対応 (v0.0.1 Preview)

リリース対応を行う。
今まで開発用で進めていたデータや環境を整理する。

## 必須対応

### License・帰属表示 [solved]

- 機械学習の対応が遅くなるので、License整理は最後に行う
- THIRD_PARTY_LICENSES または NOTICE ファイルを作成し、全依存のライセンスをまとめる
    - Apache-2.0 の配布条件 4(d) として帰属表示が必要
- 対象:
    - ORT / ONNX Runtime (MIT, Microsoft)
    - imgui / imgui-sys (MIT + Apache-2.0 dual)
    - フォント: Dokdo (OFL), Roboto (Apache-2.0)
    - Vulkan SDK 関連
- 機械学習のトレーニングデータ由来のライセンス確認
    - CMU MoCap: free
    - 100Style: 要確認だが、AnimationModelTrainingリポジトリで確認していた時は使用可能という調査結果あり
- 特に機械学習に用いている ONNX モデルファイルを含めているので、ライセンス表記が必要

### Cargo.toml メタデータ [solved]

- `license`, `authors`, `description`, `repository` を設定
- `version` をリリースバージョンに合わせる
- `name` をアプリ名リネーム（下記）と整合させる

### アプリ起動方法・配布パッケージ

- `cargo build --release` で .exe ファイル作成
- 一旦アイコンテクスチャは無くても良い

#### 配布パッケージ構成

zip 配布。PowerShell スクリプト (`scripts/package.ps1`) で自動化する。

```
thyllore-animation-v0.0.1-preview-win-x64/
├── thyllore-animation.exe                    # Release ビルド
├── onnxruntime.dll                           # vendor/onnxruntime/ からコピー
├── onnxruntime_providers_shared.dll           # 同上
├── ml/
│   └── model/
│       └── curve_copilot.onnx                # ml/model/ からそのままコピー
├── assets/
│   ├── shaders/*.spv                         # build.rs がコンパイル済み (26個)
│   ├── fonts/
│   │   ├── Roboto-Regular.ttf
│   │   ├── mplus-1p-regular.ttf
│   │   ├── Dokdo-Regular.ttf
│   │   ├── LICENSE-Roboto.txt
│   │   └── LICENSE-Dokdo.txt
│   ├── textures/
│   │   ├── lightIcon.png
│   │   └── white.png
│   └── models/stickman/stickman.glb          # サンプルモデル (任意)
├── LICENSE
├── THIRD_PARTY_LICENSES
└── README.md
```

ML モデル解決順序 (`resolve_curve_copilot_model_path`): [solved]
1. `ml/model/curve_copilot.onnx` — ローカル (配布パッケージ同梱)
2. `../SharedData/exports/curve_copilot.onnx` — 開発時 SharedData
3. `../SharedData/exports/curve_copilot_*.onnx` — 日付付きバージョン
4. `assets/ml/curve_copilot_dummy.onnx` — フォールバック (ダミー)

#### 対応事項

- [x] `paths.rs` の ML モデル解決パスに `ml/model/curve_copilot.onnx` を追加
- [ ] ORT DLL の同梱: `vendor/onnxruntime/.../lib/` から exe と同階層にコピー
  - Windows は exe と同ディレクトリの DLL を自動ロードする
- [ ] `scripts/package.ps1` を作成
  - `cargo build --release` → 必要ファイル収集 → zip 作成
- [x] ml 機能なしビルドの手順を README に記載 (`--no-default-features`)
- [x] サンプルモデルのライセンス確認
  - stickman: 自作 or ライセンス確認済みか要確認
  - サードパーティモデル (fuse-woman, phoenix-bird 等) は配布に含めない

### ReadMe 整理 [solved]

- 最後に行う
- わかりやすい Readme 記述（現在1行のみ）
- 他 OSS のフォーマットに従う
- 最低限: プロジェクト概要、スクリーンショット、動作環境、インストール手順、使い方

## 推奨対応

### CHANGELOG

- プレビュー版でも「何ができるか」の記録
- GitHub Releases の本文でも代用可

### panic/unwrap の整理 [solved]

- 現在 30 ファイルで `unwrap()` 使用中
- release ビルドでクラッシュするとユーザー体験が悪い
- 最低限 main.rs や起動パスのものを `expect` + エラーメッセージに変更

### release ビルドの動作確認 [solved]

- debug と release で挙動が異なることがある
    - 最適化で浮動小数点の差異
    - shader validation の有無
- release ビルドで一通りの操作を確認する

### バージョン表示 [solved]

- アプリ内のどこかに `v0.0.1-preview` を表示する
- 不具合報告時にバージョン特定が容易になる

### assets/ 依存の解消 [solved]
- git 管理されていないので、ファイルがない場合にクラッシュする可能性がある
- 依存の解消
- テスト追加

## 改善対応

### デバッグ window の改善 [solved]

- メッセージウィンドウを追加する
- パラメータ調整項目は別 window にする
    - scene view に overlay でトグルを表示する形を検討
- 不要なものは削除

### ログの改善

#### 調査結果: 主要エンジンのリリースビルド時のログ戦略 [solved]

| エンジン | 方式 | リリースで残るレベル |
|---------|------|-------------------|
| Unreal Engine | コンパイルアウト (`NO_LOGGING`, `UE_BUILD_SHIPPING`) | Error / Warning / Display のみ |
| Bevy (Rust) | `release_max_level_warn` feature でゼロコスト除去 | Warn / Error のみ |
| Blender | ランタイム verbosity 制御 (Clog) | Warn / Error が実質有効 |
| Unity | 自動除去なし（手動でラッパーが必要） | 全ログ残存 |
| Godot | `is_debug_build()` での条件分岐 | 全ログ残存 |

- Unreal / Bevy が業界のベストプラクティス: リリースでは Warn 以上のみ、Debug/Info はコンパイル除去
- ML推論ログはリリースでは Warn 以上のみ残すのが標準 (Unreal NNE, Bevy 共通)

#### 調査結果: ログファイルの保存場所

現状はプロジェクトディレクトリ内の `log/` にログを書いている。
主要アプリはユーザーディレクトリ配下の標準パスを使用している。

| アプリ | Windows ログパス |
|-------|----------------|
| Unreal (エディタ) | `<Project>\Saved\Logs\` |
| Unreal (パッケージ済) | `%LOCALAPPDATA%\<GameName>\Saved\Logs\` |
| Unity (エディタ) | `%LOCALAPPDATA%\Unity\Editor\` |
| Unity (ビルド済) | `%LOCALAPPDATA%Low\<Company>\<Product>\` |
| Blender | `%APPDATA%\Blender Foundation\Blender\<ver>\` |
| Maya | `%LOCALAPPDATA%\Autodesk\Maya\<ver>\logs\` |
| VS Code | `%APPDATA%\Code\logs\` |
| Chrome | `%LOCALAPPDATA%\Google\Chrome\User Data\` |

OS別の標準ログパス:

| OS | 推奨パス |
|---|---------|
| Windows | `%LOCALAPPDATA%\<AppName>\logs\` |
| macOS | `~/Library/Logs/<AppName>/` |
| Linux | `$XDG_STATE_HOME/<appname>/` (`~/.local/state/<appname>/`) |

v0.0.1 は GitHub zip 配布のため、ユーザーが展開先を自由に選べる。
`Program Files` へのインストールではないので、アプリ同梱の `log/` で権限問題は起きない。
`%LOCALAPPDATA%` への移行はインストーラー対応時に検討する。

#### 対応方針

- ~~現在の `crate::log!()` は単一レベルで debug/release の区別がない~~
- Bevy / Unreal に倣い、ログレベルを導入する: **(solved)**
    - `log_error!` → Error（リリースでも残す）
    - `log_warn!` → Warning（リリースでも残す）
    - `log!` → Info 相当（リリースでは `#[cfg(debug_assertions)]` で除去）
- CurveCopilot 等の数値診断ログは Info 相当 → リリースでは除去 **(solved)**
- v0.0.1: ログ出力先は現状の `log/` を維持（zip 配布のため問題なし）
- 将来（インストーラー対応時）: `%LOCALAPPDATA%\<AppName>\logs\` に移行
    - `directories` クレートでクロスプラットフォーム対応

### 描画順の整理 [solved]

- グリッドが最前面になっているのを修正 **(solved: composite/tonemapパスでgl_FragDepth書き込み追加)**
- y軸描画はオプションにする
    - 開発時は合った方が良い
    - リリース時はない方が良い
- bone の描画もオプションにする

### スケール再確認 [solved]
- ログにスケールが小さすぎる記述あり
- Blenderとスケールを比較するテストを書く

### 機械学習

- 可能な限りの改善
    - v3モデルのconfidence二重sigmoid問題を修正 **(solved)**
    - モデルはまだ6/100 epoch学習途中。トレーニング進行で精度向上見込み
- 実用レベルでなければ非アクティブも検討だが、信用値が低い場合は提案されないのでこのままでも良い

### アプリ名リネーム [solved]

- 正式名称にする。候補は以下
    - Oxy Animation
    - Chloro Engine
    - Chloroplast Engine
    - pecta animation
    - xylo animation
    - thyllis animation
    - thyllore animation
        - 「Diorのリズム」と「Thylakoid（光合成の核）」
    - xyllis animation

### UI の改善

- ボタンを大きくして押しやすくする
- 画面内に説明を追加、もしくは Readme 拡充

### 不具合修正

- 見つけたものから修正する

### 整理整頓

- DebugState の整理
- ECSリソースが適切かを確認

## 将来対応

- CI / GitHub Actions（自動ビルド・テスト）
- インストーラー対応
- gameview camera