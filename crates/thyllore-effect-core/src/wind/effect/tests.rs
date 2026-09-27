use crate::test_support::scene::*;
use crate::wind::*;
use thyllore_scene_core::{find_ui_param, UiKind};

const FIXTURE_DIR: &str = "src/wind/effect";

#[test]
fn test_default_json_matches_fixture() {
    assert_matches_fixture(
        &default_json::<WindTornadoEffect>(),
        include_str!("scene_format_default.json"),
        &format!("{FIXTURE_DIR}/scene_format_default.json"),
    );
}

#[test]
fn test_ui_params_match_fixture() {
    assert_matches_fixture(
        &ui_params_json(&WIND_UI_PARAMS),
        include_str!("ui_params.json"),
        &format!("{FIXTURE_DIR}/ui_params.json"),
    );
}

#[test]
fn test_parameter_ownership_matches_fixture() {
    assert_matches_fixture(
        &ownership_json(&WIND_PARAMETER_OWNERSHIP),
        include_str!("parameter_ownership.json"),
        &format!("{FIXTURE_DIR}/parameter_ownership.json"),
    );
}

#[test]
fn test_tables_are_consistent() {
    assert_tables_are_consistent::<WindTornadoEffect>(
        &WIND_SCALAR_PARAMS,
        &WIND_UI_PARAMS,
        WIND_PARAMETER_OWNERSHIP.len(),
    );
    assert_eq!(
        <WindTornadoEffect as thyllore_scene_core::SceneComponent>::TYPE_KEY,
        "wind_tornado"
    );
}

#[test]
fn test_ron_struct_syntax_roundtrip() {
    let mut effect = WindTornadoEffect::default();
    effect.column_height = 3.5;
    effect.wall_width_q = 0.2;

    let text =
        ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new()).expect("ron serialize");
    let restored: WindTornadoEffect = ron::from_str(&text).expect("ron deserialize");
    assert_eq!(restored.column_height, 3.5);
    assert_eq!(restored.wall_width_q, 0.2);
}

#[test]
fn test_ui_param_groups_cover_every_wind_group_in_display_order() {
    assert_eq!(
        groups_in_display_order(&WIND_UI_PARAMS),
        ["shape", "density", "motion", "eddy", "look"]
    );
}

#[test]
fn test_albedo_is_a_color_and_serializes_as_one_vector() {
    assert_eq!(
        find_ui_param(&WIND_UI_PARAMS, "albedo").map(|p| p.kind),
        Some(UiKind::Color)
    );
    let value = serde_json::to_value(WindTornadoEffect::default()).expect("serialize");
    let object = value.as_object().expect("flat object");
    assert!(object["albedo"].is_array());
    assert!(!object.contains_key("albedo_r"));
}

#[test]
fn test_overwrite_persisted_fields_keeps_runtime_state() {
    let mut loaded = WindTornadoEffect::default();
    loaded.column_height = 5.0;
    loaded.time = 5.0;

    let mut target = WindTornadoEffect::default();
    target.time = 2.5;

    overwrite_wind_persisted_fields(&mut target, &loaded);
    assert_eq!(target.column_height, 5.0);
    assert_eq!(target.time, 2.5);
}

#[test]
fn test_primary_params_are_exactly_the_specified_8() {
    let mut primary = primary_names(&WIND_UI_PARAMS);
    primary.sort_unstable();
    assert_eq!(
        primary,
        [
            "albedo",
            "circulation",
            "column_height",
            "density",
            "eddy_amplitude",
            "rise_duration",
            "time_scale",
            "wall_radius_base",
        ]
    );
}
