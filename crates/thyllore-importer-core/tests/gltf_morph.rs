use std::path::{Path, PathBuf};

use thyllore_importer_core::ModelLoadResult;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/morph_quad.gltf")
}

fn load_fixture() -> ModelLoadResult {
    let path = fixture_path().to_string_lossy().to_string();
    let result = unsafe { thyllore_importer_core::gltf::load_gltf_file(&path) }.expect("load glTF");
    ModelLoadResult::from_gltf(result)
}

#[test]
fn gltf_morph_targets_become_named_sparse_channels() {
    let result = load_fixture();

    assert_eq!(result.meshes.len(), 1);
    let morph = &result.meshes[0].morph;
    assert_eq!(morph.source_mesh, "face");
    let names: Vec<&str> = morph.channels.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["smile", "blink"]);

    let smile = &morph.channels[0];
    assert_eq!(smile.position_deltas.len(), 1);
    assert_eq!(smile.position_deltas[0].vertex_index, 2);
    assert_eq!(smile.position_deltas[0].delta, [0.0, 0.5, 0.0]);

    let blink = &morph.channels[1];
    assert_eq!(blink.position_deltas.len(), 1);
    assert_eq!(blink.position_deltas[0].vertex_index, 1);
}

#[test]
fn gltf_weight_animation_becomes_morph_channels_on_a_clip() {
    let result = load_fixture();

    assert_eq!(result.clips.len(), 1);
    let clip = &result.clips[0];
    assert_eq!(clip.name, "expression");
    assert_eq!(clip.duration, 2.0);
    assert_eq!(clip.morph_channels.len(), 2);

    let smile = &clip.morph_channels[0];
    assert_eq!(smile.source_mesh, "face");
    assert_eq!(smile.channel, "smile");
    let smile_values: Vec<f32> = smile.keyframes.iter().map(|k| k.value).collect();
    assert_eq!(smile_values, vec![0.0, 1.0, 0.5]);

    let blink_values: Vec<f32> = clip.morph_channels[1]
        .keyframes
        .iter()
        .map(|k| k.value)
        .collect();
    assert_eq!(blink_values, vec![0.0, 0.0, 1.0]);
}
