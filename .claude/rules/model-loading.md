---
paths:
  - "src/app/model_loader.rs"
  - "src/loader/**"
---

# Adding New Models

To load a new model, modify `App::load_model()`:

- For glTF: Update `model_path` variable
- For FBX: Update `model_path_fbx` variable
- Ensure textures are in the same directory or adjust texture paths
