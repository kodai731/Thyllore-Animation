use crate::test_support::scene::*;
use crate::water::*;
use thyllore_scene_core::{find_ui_param, UiKind};

#[test]
fn test_tables_are_consistent() {
    assert_tables_are_consistent::<WaterTorusEffect>(
        &WATER_SCALAR_PARAMS,
        &WATER_UI_PARAMS,
        WATER_PARAMETER_OWNERSHIP.len(),
    );
    assert_eq!(
        <WaterTorusEffect as thyllore_scene_core::SceneComponent>::TYPE_KEY,
        "water_torus"
    );
}

#[test]
fn test_ron_struct_syntax_roundtrip() {
    let mut effect = WaterTorusEffect::default();
    effect.major_radius = 2.5;
    effect.absorption = [0.1, 0.2, 0.3];

    let text =
        ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new()).expect("ron serialize");
    let restored: WaterTorusEffect = ron::from_str(&text).expect("ron deserialize");
    assert_eq!(restored.major_radius, 2.5);
    assert_eq!(restored.absorption, [0.1, 0.2, 0.3]);
}

#[test]
fn test_ui_param_groups_cover_every_water_group_in_display_order() {
    assert_eq!(
        groups_in_display_order(&WATER_UI_PARAMS),
        ["shape", "optics", "flow", "wave", "lighting", "look"]
    );
}

#[test]
fn test_color_kinds_are_assigned_to_absorption_and_tint() {
    let kind = |name| find_ui_param(&WATER_UI_PARAMS, name).map(|p| p.kind);
    assert_eq!(kind("absorption"), Some(UiKind::Absorption));
    assert_eq!(kind("tint"), Some(UiKind::Color));
    assert_eq!(kind("ior"), Some(UiKind::Scalar));

    let value = serde_json::to_value(WaterTorusEffect::default()).expect("serialize");
    let object = value.as_object().expect("flat object");
    assert!(object["absorption"].is_array());
    assert!(!object.contains_key("absorption_r"));
}

#[test]
fn test_overwrite_persisted_fields_keeps_runtime_state() {
    let mut loaded = WaterTorusEffect::default();
    loaded.major_radius = 5.0;
    loaded.time = 5.0;

    let mut target = WaterTorusEffect::default();
    target.time = 2.5;

    overwrite_water_persisted_fields(&mut target, &loaded);
    assert_eq!(target.major_radius, 5.0);
    assert_eq!(target.time, 2.5);
}

#[test]
fn test_primary_params_are_exactly_the_specified_7() {
    let mut primary = primary_names(&WATER_UI_PARAMS);
    primary.sort_unstable();
    assert_eq!(
        primary,
        [
            "flow_longitudinal",
            "light_intensity",
            "reflect_strength",
            "time_scale",
            "tint",
            "wave_amplitude",
            "wave_frequency",
        ]
    );
}
