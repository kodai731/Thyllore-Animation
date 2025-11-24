- 目標
    - fbxファイルをアニメーションする
    - これ以降は次の段階だが、必要に応じて実装する
    - fbxファイルをskinningする
    - fbxファイルをskinningアニメーションする
    
- 問題
    - アニメーションが上手く動作せず、動いていないように見える

- アプローチ
    - Blender と ufbx では同じfbxファイル（stickman_bin.fbx）を読み込むと時刻0の正しいポーズが表示され、アニメーションもされる
    - Blender のスクリーンショットは log/スクリーンショット_Blender.png
    - ufbx のスクリーンショットは log/スクリーンショット_ufbx.png
    - Blender の行列は log/Blender_Matrix.txt に保存されているので、参照すること
    - ufbx の実装は ../ufbx/ にリポジトリをクローンしているので、参照すること
    - そのほかに役立ちそうなC++とRustのリポジトリは memo.txt に記載している

- ログ
    - log/ にログを保存しています。不具合修正や分析に使用してください
    - log/Blender_Matrix.txt にBlenderの行列を保存しています
    - log/log_N.txt にローテーションされたlogが記録される仕組みです
    - 不具合発生時には原因究明のため、logに記載する内容を追加することを検討してください

- 重要
    - CLAUDE.md にプロジェクト概要があるので、参照してください
    - 
