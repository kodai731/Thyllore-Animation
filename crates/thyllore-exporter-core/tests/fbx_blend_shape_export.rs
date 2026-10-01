#![cfg(test)]

use std::path::Path;

use thyllore_anim_core::editable::{curve_add_keyframe, EditableAnimationClip};
use thyllore_anim_core::Skeleton;

fn copy_morph_cube_fixture_to_temp() -> std::path::PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../thyllore-importer-core/tests/data/morph_cube.fbx.txt");
    let temp = std::env::temp_dir().join("morph_cube_export_source.fbx");
    std::fs::copy(&fixture, &temp).expect("copy morph_cube fixture");
    temp
}

#[test]
fn test_blend_shapes_survive_full_fbx_export() {
    let source_path = copy_morph_cube_fixture_to_temp();
    let source_model =
        thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(source_path.to_str().unwrap())
            .expect("load morph_cube");

    let export_path = std::env::temp_dir().join("morph_cube_exported.fbx");
    thyllore_exporter_core::systems::fbx::export_full_fbx(
        &source_model,
        None,
        &Skeleton::default(),
        &export_path,
    )
    .expect("export morph_cube");

    let exported_model =
        thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(export_path.to_str().unwrap())
            .expect("reload exported morph_cube");
    assert_eq!(exported_model.fbx_data.len(), 1, "expected one mesh");

    let morph = &exported_model.fbx_data[0].morph;
    let channel_names: Vec<&str> = morph.channels.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(channel_names, ["smile", "blink"]);

    let expected_deltas = [[0.1_f32, 0.0, 0.0], [0.0, -0.1, 0.0]];
    for (channel, expected) in morph.channels.iter().zip(expected_deltas) {
        assert_eq!(
            channel.position_deltas.len(),
            1,
            "{} delta count",
            channel.name
        );
        let delta = channel.position_deltas[0].delta;
        for axis in 0..3 {
            assert!(
                (delta[axis] - expected[axis]).abs() < 1e-4,
                "{} delta axis {}: expected {}, got {}",
                channel.name,
                axis,
                expected[axis],
                delta[axis]
            );
        }
    }

    std::fs::remove_file(&source_path).ok();
    std::fs::remove_file(&export_path).ok();
}

#[test]
fn test_morph_track_exports_as_deform_percent_curve() {
    let source_path = copy_morph_cube_fixture_to_temp();
    let source_model =
        thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(source_path.to_str().unwrap())
            .expect("load morph_cube");

    let mut clip = EditableAnimationClip::new(0, "smile".to_string());
    let smile_curve = &mut clip.get_or_add_morph_track("Cube", "smile").curve;
    curve_add_keyframe(smile_curve, 0.0, 0.0);
    curve_add_keyframe(smile_curve, 1.0, 1.0);

    let export_path = std::env::temp_dir().join("morph_cube_weight_anim_exported.fbx");
    thyllore_exporter_core::systems::fbx::export_full_fbx(
        &source_model,
        Some(&clip),
        &Skeleton::default(),
        &export_path,
    )
    .expect("export morph_cube with morph track");

    let scene = ufbx::load_file(export_path.to_str().unwrap(), ufbx::LoadOpts::default())
        .expect("reload exported morph_cube");
    let find_channel = |name: &str| {
        scene
            .blend_channels
            .iter()
            .find(|channel| channel.element.name == name)
            .unwrap_or_else(|| panic!("{name} channel missing"))
    };
    let smile = find_channel("smile");
    let blink = find_channel("blink");

    for (time, expected_smile) in [(0.0, 0.0), (1.0, 1.0)] {
        let smile_weight = ufbx::evaluate_blend_weight(&scene.anim, smile, time);
        let blink_weight = ufbx::evaluate_blend_weight(&scene.anim, blink, time);
        assert!(
            (smile_weight - expected_smile).abs() < 1e-4,
            "smile at {time}: expected {expected_smile}, got {smile_weight}"
        );
        assert!(
            blink_weight.abs() < 1e-4,
            "blink at {time}: expected 0, got {blink_weight}"
        );
    }

    std::fs::remove_file(&source_path).ok();
    std::fs::remove_file(&export_path).ok();
}
