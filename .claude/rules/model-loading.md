---
paths:
  - "src/app/model/**"
  - "src/app/scene_model.rs"
  - "src/app/init/instance.rs"
  - "crates/thyllore-importer-core/**"
---

# Model Loading

- Importers (glTF / FBX / PNG) live in `crates/thyllore-importer-core` and return pure model-core + anim-core
  types; `src/loader/` only re-exports them.
- `App::load_model(path)` / `App::load_model_additive(path)` (`src/app/scene_model.rs`) are the runtime entry
  points; `src/app/model/` turns the importer result into `AssetStorage` entries, GPU meshes and acceleration
  structures, then runs `ModelLoadHooks` (`src/hooks/model_load.rs`) without naming a domain.
- The startup model comes from the scene file: `--batch-scene <path>` or `assets/scenes/default.scene.ron`
  (`determine_startup_model` in `src/app/init/instance.rs`). There is no hard-coded model path; to change the
  default model, edit the scene file, not the code.
- Sample models live under `assets/models/<name>/`; textures are resolved relative to the model file
  (`src/app/model/texture.rs`).
- At runtime a model is loaded through `AppCommand::LoadModel` / `LoadModelAdditive`, applied by `App` in
  `src/app/command.rs`; the platform layer only records the command.
