---
paths:
  - "src/ecs/systems/model_load_systems/**"
  - "src/app/scene_model.rs"
  - "src/app/init/instance.rs"
  - "crates/thyllore-importer-core/**"
---

# Model Loading

- Importers (glTF / FBX / PNG) live in `crates/thyllore-importer-core` and return pure model-core + anim-core
  types; `src/loader/` only re-exports them.
- The FBX scene data both the importer and the exporter use (`FbxModel`, `FbxData`, `BoneNode`, ...) lives in
  `crates/thyllore-file-format-core`; `thyllore-exporter-core` depends on that crate and names
  `thyllore-importer-core` only in tests, never the other way round.
- `App::load_model(path)` / `App::load_model_additive(path)` (`src/app/scene_model.rs`) are the runtime entry
  points; `src/ecs/systems/model_load_systems/` turns the importer result into `AssetStorage` entries, GPU meshes and acceleration
  structures, then runs `ModelLoadHooks` (`src/hooks/model_load.rs`) without naming a domain.
- What does not need `World` or Vulkan lives in the crates: the mesh texture choice
  (`thyllore_importer_core::resolve_mesh_texture`), the clip kept across a load
  (`thyllore_anim_core::editable::select_kept_clip_id` / `is_user_clip_selected`) and the animation type
  (`classify_imported_animation`).
- The startup model comes from the scene file: `--batch-scene <path>` or `assets/scenes/default.scene.ron`
  (`determine_startup_model` in `src/app/init/instance.rs`). There is no hard-coded model path; to change the
  default model, edit the scene file, not the code.
- Sample models live under `assets/models/<name>/`; textures are resolved relative to the model file
  (`thyllore-importer-core` `texture/search.rs`, `texture/resolve.rs`).
- At runtime a model is loaded through `SceneLoadCommand::LoadModel` / `LoadModelAdditive`, pushed to
  `SceneLoadQueue` and applied by `App` in `src/app/command.rs`.
