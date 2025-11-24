- 目標
    - fbxファイルをアニメーションする
    - これ以降は次の段階
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


