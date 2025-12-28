use std::path::Path;

#[test]
fn test_gltf_model_files_exist() {
    let model_paths = [
        "assets/models/stickman/stickman.glb",
    ];

    for path in &model_paths {
        assert!(
            Path::new(path).exists(),
            "glTF model file should exist: {}",
            path
        );
    }
}

#[test]
fn test_fbx_model_files_exist() {
    let model_paths = [
        "assets/models/phoenix-bird/source/fly.fbx",
        "assets/models/stickman/stickman.fbx",
        "assets/models/stickman/stickman_bin.fbx",
    ];

    for path in &model_paths {
        assert!(
            Path::new(path).exists(),
            "FBX model file should exist: {}",
            path
        );
    }
}

#[test]
fn test_gltf_file_not_empty() {
    use std::fs;

    let path = "assets/models/stickman/stickman.glb";
    let metadata = fs::metadata(path).expect("Failed to read model file metadata");
    assert!(metadata.len() > 0, "glTF model file should not be empty");
}

#[test]
fn test_fbx_file_not_empty() {
    use std::fs;

    let path = "assets/models/phoenix-bird/source/fly.fbx";
    let metadata = fs::metadata(path).expect("Failed to read model file metadata");
    assert!(metadata.len() > 0, "FBX model file should not be empty");
}

#[test]
fn test_nonexistent_model_file() {
    let path = "assets/models/nonexistent_model.glb";
    assert!(
        !Path::new(path).exists(),
        "Nonexistent model file should not exist"
    );
}

#[test]
fn test_model_directory_structure() {
    assert!(Path::new("assets/models").exists(), "models directory should exist");
    assert!(Path::new("assets/models/stickman").exists(), "stickman directory should exist");
    assert!(Path::new("assets/models/phoenix-bird").exists(), "phoenix-bird directory should exist");
}

#[test]
fn test_texture_files_exist() {
    let texture_paths = [
        "assets/textures/white.png",
    ];

    for path in &texture_paths {
        assert!(
            Path::new(path).exists(),
            "Texture file should exist: {}",
            path
        );
    }
}

#[test]
fn test_phoenix_bird_textures_exist() {
    let texture_dir = "assets/models/phoenix-bird/textures";
    assert!(
        Path::new(texture_dir).exists(),
        "Phoenix bird textures directory should exist"
    );

    let texture_files = [
        "Tex_Ride_FengHuang_01a_D_A.tga.png",
        "Tex_Ride_FengHuang_01a_E.tga.png",
        "Tex_Ride_FengHuang_01b_D_A.tga.png",
        "Tex_Ride_FengHuang_01b_E.tga.png",
    ];

    for file in &texture_files {
        let path = format!("{}/{}", texture_dir, file);
        assert!(
            Path::new(&path).exists(),
            "Phoenix bird texture should exist: {}",
            path
        );
    }
}

#[test]
fn test_viking_room_model_exists() {
    let model_path = "assets/models/VikingRoom/viking_room.obj";
    let texture_path = "assets/models/VikingRoom/viking_room.png";

    assert!(
        Path::new(model_path).exists(),
        "Viking room model should exist"
    );
    assert!(
        Path::new(texture_path).exists(),
        "Viking room texture should exist"
    );
}
