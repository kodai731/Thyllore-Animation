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
  types; the engine uses them as `thyllore_importer_core::`.
- The FBX scene data both the importer and the exporter use (`FbxModel`, `FbxData`, `BoneNode`, ...) lives in
  `crates/thyllore-file-format-core`; `thyllore-exporter-core` depends on that crate and names
  `thyllore-importer-core` only in tests, never the other way round.
- A model is a scene entity: `ModelSource { path }` (`src/ecs/component/model/source.rs`, `scene_owner!`, type
  key `model_source`) names the file, and the model's mesh entities are that entity's children. The path is
  relative to the scene's assets directory (`SceneState::assets_dir`: two levels above the scene file, `assets`
  without a scene) or absolute when the file lies outside it. A generated mesh has no `ModelSource`; its entity
  carries `GeneratedGlb` (the bytes it was built from) and is not persisted.
- Whether a model is loaded is computed, never stored: a `ModelSource` entity without `Children` is not loaded.
  `request_pending_model_loads` (`src/ecs/systems/model_load_systems/source.rs`, a `dispatch_prep_hook!`) pushes
  `SceneLoadCommand::LoadModel { entity }` for each such entity, so startup and runtime loads share one path.
  To load a model, spawn the entity with `spawn_model_source`; never push the command or call the loader.
- `ModelLoadMark` (`src/ecs/component/model/load_mark.rs`, not persisted) holds what `Children` cannot say:
  `Added` (`spawn_model_source`: the user added the model to the open scene), `ReloadRequested`
  (`request_model_reload`: the file is read again while the entity keeps its meshes) and `Failed`. The loader
  removes the mark when it starts (`begin_model_load`).
- `SceneLoadCommand::LoadModel { entity }` (`src/app/command.rs`) runs `load_model_keeping_scene` through
  `App::rebuild_scene` (`src/app/scene_rebuild.rs`). The loader (`keep_scene.rs`) despawns every other
  `ModelSource` entity (one scene holds one model), captures the scene's entities, resets the world and uploads
  the meshes (`replace.rs::upload_scene_model`), restores the entities, attaches the meshes and the animation to
  the restored model entity (`attach_scene_model`), runs `ModelLoadHooks` (`src/hooks/model_load.rs`) without
  naming a domain, and rebuilds the acceleration structures once.
- A failed load leaves no empty model entity, with one exception (`fail_model_load`): the model a scene file
  names, alone in the scene, keeps its `ModelSource` entity marked `ModelLoadMark::Failed`, so it is not requested
  again and saving the scene still names the model. A model the user added (`Added`: drop, open button, batch
  `load_model=`) and a model that fails beside a loaded one lose their entity. The error goes to the message log.
  A file without meshes is refused the same way. A failed reload leaves the meshes of the last load in place. The
  clip the scene asked of the model (`TimelineState::requested_clip_name`) is discarded with the failure.
- A reload starts on the model clip the model was playing (`request_played_clip_of_reloading_model`).
- Save scene writes the open scene's own file (`SceneState::current_scene_path`, the default scene without one):
  a `ModelSource` path is relative to that scene's assets directory.
- The current model is found, not remembered: `find_loaded_model_entity` / `find_loaded_model_file`
  (`source.rs`) and `find_model_path` return the loaded model's entity and file. Sidecars, the expression
  library, the texture remap and exports key on that path; `ModelState` holds only what the importer reported
  (`has_skinned_meshes`, `imported_humanoid`).
- What does not need `World` or Vulkan lives in the crates: the mesh texture choice
  (`thyllore_importer_core::resolve_mesh_texture`), the clip kept across a load
  (`thyllore_anim_core::editable::select_kept_clip_id`) and the animation type
  (`classify_imported_animation`). Whether the selected clip is the user's own is a `ClipLibrary` method
  (`is_user_clip_selected`, `src/ecs/resource/editor/clip_library.rs`).
- The engine starts from a scene, not from a model: `load_startup_scene` (`src/app/init/startup_scene.rs`) reads
  `--batch-scene <path>` or `assets/scenes/default.scene.ron`, and `apply_startup_scene`
  (`src/ecs/systems/world/startup_scene.rs`) restores its resources and entities. Neither it, `src/scene/` nor
  `src/app/init/` knows what a model is: the scene's `model_source` entity is restored like any other owner and
  the first dispatch requests its load. There is no hard-coded model path; to change the default model, edit
  the scene file, not the code.
- Scene files are version 9. A version 8 file keeps its model in a top-level `model: (path: ..)` block and is
  rejected; `assets/scenes/` is not tracked by git, so every clone converts its own files once:
  `python3 scripts/scene/migrate_v8_to_v9.py --backup assets/scenes_v8_backup assets/scenes/*.scene.ron`.
  A tool that writes a scene for a model uses `render_model_scene` (`tools/engine_harness.py`).
- Sample models live under `assets/models/<name>/`; textures are resolved relative to the model file
  (`thyllore-importer-core` `texture/search.rs`, `texture/resolve.rs`).
- At runtime the UI sends `CameraEvent::LoadModel { path }`, whose apply calls `spawn_model_source`; the load
  follows on the next dispatch through `SceneLoadQueue`, applied by `App` in `src/app/command.rs`.
