use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_path() -> PathBuf {
    workspace_root().join("crates/thyllore-importer-core/tests/data/morph_cube.fbx.txt")
}

fn copy_fixture_to_temp() -> PathBuf {
    let mut temp = std::env::temp_dir();
    temp.push("morph_cube_test.fbx");
    let content = fs::read(fixture_path()).expect("read fixture");
    let mut file = fs::File::create(&temp).expect("create temp file");
    file.write_all(&content).expect("write temp file");
    temp
}

#[test]
fn test_blend_shape_import() {
    let temp_path = copy_fixture_to_temp();
    let path_str = temp_path.to_string_lossy().to_string();

    let fbx_model =
        thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(&path_str).expect("load FBX");

    assert_eq!(fbx_model.fbx_data.len(), 1, "expected one mesh");

    let morph = &fbx_model.fbx_data[0].morph;
    let channel_names: Vec<&str> = morph.channels.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(channel_names, ["smile", "blink"], "channel names in order");

    let smile = &morph.channels[0];
    assert_eq!(
        smile.position_deltas.len(),
        1,
        "smile channel position delta count matches fixture Indexes count"
    );

    let blink = &morph.channels[1];
    assert_eq!(
        blink.position_deltas.len(),
        1,
        "blink channel position delta count matches fixture Indexes count"
    );

    let smile_delta = &smile.position_deltas[0];
    assert!(
        (smile_delta.delta[0] - 0.1).abs() < 1e-4,
        "smile delta x matches fixture: got {}",
        smile_delta.delta[0]
    );
    assert!(
        smile_delta.delta[1].abs() < 1e-4,
        "smile delta y is zero: got {}",
        smile_delta.delta[1]
    );
    assert!(
        smile_delta.delta[2].abs() < 1e-4,
        "smile delta z is zero: got {}",
        smile_delta.delta[2]
    );

    let blink_delta = &blink.position_deltas[0];
    assert!(
        blink_delta.delta[0].abs() < 1e-4,
        "blink delta x is zero: got {}",
        blink_delta.delta[0]
    );
    assert!(
        (blink_delta.delta[1] - (-0.1)).abs() < 1e-4,
        "blink delta y matches fixture: got {}",
        blink_delta.delta[1]
    );
    assert!(
        blink_delta.delta[2].abs() < 1e-4,
        "blink delta z is zero: got {}",
        blink_delta.delta[2]
    );

    assert!(
        smile.normal_deltas.is_empty(),
        "smile normal_deltas should be empty"
    );
    assert!(
        blink.normal_deltas.is_empty(),
        "blink normal_deltas should be empty"
    );

    fs::remove_file(&temp_path).ok();
}
