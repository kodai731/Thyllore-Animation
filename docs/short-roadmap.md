# リリース対応 (v0.0.1 Preview)

リリース対応を行う。
今まで開発用で進めていたデータや環境を整理する。

## 必須対応

### License・帰属表示

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

### Cargo.toml メタデータ

- `license`, `authors`, `description`, `repository` を設定
- `version` をリリースバージョンに合わせる
- `name` をアプリ名リネーム（下記）と整合させる

### アプリ起動方法・配布パッケージ

- `cargo build --release` で .exe ファイル作成
- 一旦アイコンテクスチャは無くても良い
- 配布パッケージ構成を決定:
    - exe + DLL (onnxruntime.dll) + assets + shaders + ML モデル + LICENSE 群
    - zip 配布を想定
- ORT DLL の同梱方法を確立する（または ml 機能なしビルドの手順を README に記載）

### ReadMe 整理

- 最後に行う
- わかりやすい Readme 記述（現在1行のみ）
- 他 OSS のフォーマットに従う
- 最低限: プロジェクト概要、スクリーンショット、動作環境、インストール手順、使い方

## 推奨対応

### CHANGELOG

- プレビュー版でも「何ができるか」の記録
- GitHub Releases の本文でも代用可

### panic/unwrap の整理

- 現在 30 ファイルで `unwrap()` 使用中
- release ビルドでクラッシュするとユーザー体験が悪い
- 最低限 main.rs や起動パスのものを `expect` + エラーメッセージに変更

### release ビルドの動作確認

- debug と release で挙動が異なることがある
    - 最適化で浮動小数点の差異
    - shader validation の有無
- release ビルドで一通りの操作を確認する

### バージョン表示

- アプリ内のどこかに `v0.0.1-preview` を表示する
- 不具合報告時にバージョン特定が容易になる

## 改善対応

### デバッグ window の改善

- メッセージウィンドウを追加する
- パラメータ調整項目は別 window にする
    - scene view に overlay でトグルを表示する形を検討
- 不要なものは削除

### ログの改善

- 他アプリでどのようにログを収集するか調べる
- 開発モードでは手元にログを書く
- リリースモードではどうするか調べる

### 描画順の整理

- グリッドが最前面になっているのを修正
- y軸描画はオプションにする
    - 開発時は合った方が良い
    - リリース時はない方が良い
- bone の描画もオプションにする

### 機械学習

- 可能な限りの改善
- 実用レベルでなければ非アクティブも検討だが、信用値が低い場合は提案されないのでこのままでも良い

### アプリ名リネーム

- 正式名称にする。候補は以下
    - Oxy Animation
    - Chloro Engine
    - Chloroplast Engine

### UI の改善

- ボタンを大きくして押しやすくする
- 画面内に説明を追加、もしくは Readme 拡充

### 不具合修正

- 見つけたものから修正する

## 将来対応

- CI / GitHub Actions（自動ビルド・テスト）
- インストーラー対応