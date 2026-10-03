use crate::lightning::*;
use crate::test_support::scene::*;
use thyllore_scene_core::{find_ui_param, UiKind};

#[test]
fn test_tables_are_consistent() {
    assert_tables_are_consistent::<LightningEffect>(
        &LIGHTNING_SCALAR_PARAMS,
        &LIGHTNING_UI_PARAMS,
        LIGHTNING_PARAMETER_OWNERSHIP.len(),
    );
    assert_eq!(
        <LightningEffect as thyllore_scene_core::SceneComponent>::TYPE_KEY,
        "lightning"
    );
}

#[test]
fn test_ron_roundtrip_keeps_the_source_enum() {
    let mut effect = LightningEffect::default();
    effect.shape.source = LightningSource::Shell { radius: 2.5 };
    effect.branch.depth = 4;

    let text =
        ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new()).expect("ron serialize");
    let restored: LightningEffect = ron::from_str(&text).expect("ron deserialize");
    assert_eq!(
        restored.shape.source,
        LightningSource::Shell { radius: 2.5 }
    );
    assert_eq!(restored.branch.depth, 4);
}

#[test]
fn test_ui_param_groups_cover_every_lightning_group_in_display_order() {
    assert_eq!(
        groups_in_display_order(&LIGHTNING_UI_PARAMS),
        ["shape", "branch", "look", "timing"]
    );
}

#[test]
fn test_colors_and_offsets_have_their_widget_kinds() {
    let kind = |name| find_ui_param(&LIGHTNING_UI_PARAMS, name).map(|p| p.kind);
    assert_eq!(kind("look_core_color"), Some(UiKind::Color));
    assert_eq!(kind("look_rim_color"), Some(UiKind::Color));
    assert_eq!(kind("shape_end_offset"), Some(UiKind::Offset));

    let value = serde_json::to_value(LightningEffect::default()).expect("serialize");
    assert!(value["look"]["core_color"].is_array());
    assert!(value["shape"]["end_offset"].is_array());
}

#[test]
fn test_offset_aliases_write_the_end_offset_components() {
    let mut effect = LightningEffect::default();
    let set = |effect: &mut LightningEffect, name: &str, value: f32| {
        (thyllore_scene_core::find_scalar_param(&LIGHTNING_SCALAR_PARAMS, name)
            .expect(name)
            .set)(effect, value)
    };
    set(&mut effect, "shape_end_offset_x", 1.0);
    set(&mut effect, "shape_end_offset_z", -3.0);
    assert_eq!(effect.shape.end_offset, [1.0, -8.0, -3.0]);
}

#[test]
fn test_overwrite_persisted_fields_keeps_runtime_state() {
    let mut loaded = LightningEffect::default();
    loaded.look.core_intensity = 50.0;
    loaded.time = 5.0;

    let mut target = LightningEffect::default();
    target.time = 2.5;

    overwrite_lightning_persisted_fields(&mut target, &loaded);
    assert_eq!(target.look.core_intensity, 50.0);
    assert_eq!(target.time, 2.5);
}

#[test]
fn test_exactly_eight_primary_params() {
    assert_eq!(primary_names(&LIGHTNING_UI_PARAMS).len(), 8);
}
