---
paths:
  - "src/ecs/systems/model_load_systems/**"
  - "src/app/scene_rebuild.rs"
  - "src/app/init/startup_scene.rs"
  - "src/ecs/systems/world/startup_scene.rs"
  - "crates/thyllore-importer-core/**"
---

# Model Loading

- Importers (glTF / FBX / PNG) live in `crates/thyllore-importer-core` and return pure model-core + anim-core
  types; `src/loader/` only re-exports them.
- The FBX scene data both the importer and the exporter use (`FbxModel`, `FbxData`, `BoneNode`, ...) lives in
  `crates/thyllore-file-format-core`; `thyllore-exporter-core` depends on that crate and names
  `thyllore-importer-core` only in tests, never the other way round.
- `SceneLoadCommand::LoadModel` (`src/app/command.rs`) runs `load_model_keeping_scene` through
  `App::rebuild_scene` (`src/app/scene_rebuild.rs`); `src/ecs/systems/model_load_systems/` turns the importer result into `AssetStorage` entries, GPU meshes and acceleration
  structures, then runs `ModelLoadHooks` (`src/hooks/model_load.rs`) without naming a domain.
- What does not need `World` or Vulkan lives in the crates: the mesh texture choice
  (`thyllore_importer_core::resolve_mesh_texture`), the clip kept across a load
  (`thyllore_anim_core::editable::select_kept_clip_id`) and the animation type
  (`classify_imported_animation`). Whether the selected clip is the user's own is a `ClipLibrary` method
  (`is_user_clip_selected`, `src/ecs/resource/editor/clip_library.rs`).
- The engine starts from a scene, not from a model: `load_startup_scene` (`src/app/init/startup_scene.rs`) reads
  `--batch-scene <path>` or `assets/scenes/default.scene.ron`, and `apply_startup_scene`
  (`src/ecs/systems/world/startup_scene.rs`) restores it and pushes `SceneLoadCommand::LoadModel` when the scene
  names a model file (`LoadedScene.model_path`; `None` for an empty path or a generated mesh). `src/app/init/`
  never sees a model path, and `ModelState.model_path` is written only by the load itself. There is no hard-coded
  model path; to change the default model, edit the scene file, not the code.
- Sample models live under `assets/models/<name>/`; textures are resolved relative to the model file
  (`thyllore-importer-core` `texture/search.rs`, `texture/resolve.rs`).
- At runtime a model is loaded through `SceneLoadCommand::LoadModel`, pushed to
  `SceneLoadQueue` and applied by `App` in `src/app/command.rs`.
