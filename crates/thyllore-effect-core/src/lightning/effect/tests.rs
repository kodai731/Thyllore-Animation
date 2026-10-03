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
        ["shape", "branch", "look", "timing", "surround"]
    );
}

#[test]
fn test_colors_and_offsets_have_their_widget_kinds() {
    let kind = |name| find_ui_param(&LIGHTNING_UI_PARAMS, name).map(|p| p.kind);
    assert_eq!(kind("look_core_color"), Some(UiKind::Color));
    assert_eq!(kind("look_rim_color"), Some(UiKind::Color));
    assert_eq!(kind("surround_light_color"), Some(UiKind::Color));
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

#[test]
fn test_old_ron_without_surround_fields_reads_defaults() {
    let old_ron = r#"
(
    position: (0.0, 1.0, 0.0),
    rotation: (1.0, 0.0, 0.0, 0.0),
    shape: (
        end_offset: (0.0, -8.0, 0.0),
        source: (
            kind: "Point",
        ),
        strikes_per_burst: 1,
        detail_levels: 5,
        tortuosity: 0.3,
        roughness: 0.6,
        core_radius: 0.02,
        tip_radius_ratio: 0.5,
        edge_fraction: 0.3,
        end_variance: 0.0,
    ),
    branch: (
        depth: 2,
        probability: 0.55,
        count: 1.0,
        zone_start: 0.1,
        zone_end: 0.9,
        angle: 0.9,
        length_ratio: 0.45,
        radius_ratio: 0.3,
        intensity_ratio: 0.7,
    ),
    look: (
        core_intensity: 10.0,
        core_color: (1.0, 1.0, 1.0),
        rim_ratio: 8.0,
        rim_intensity: 1.5,
        rim_color: (0.15, 0.6, 1.0),
        beam_radius: 0.0,
        beam_arc_count: 8,
        flash_gain: 0.0,
        flash_radius: 2.0,
    ),
    timing: (
        seed: 0,
        burst_start: 0.08,
        burst_interval: 2.0,
        burst_jitter: 0.2,
        burst_count: 1,
        attack_time: 0.01,
        sustain_time: 0.04,
        release_time: 0.12,
        stroke_count: 1,
        stroke_interval: 0.05,
        stroke_decay: 0.6,
        flicker_amplitude: 0.25,
        flicker_period: 0.03,
        reseed_level: 2,
        reseed_period: 1.0,
        charge_ramp: 0.0,
        growth_time: 0.0,
    ),
)
"#;
    let effect: LightningEffect = ron::from_str(old_ron).expect("old RON must deserialize");
    assert_eq!(effect.position, cgmath::Vector3::new(0.0, 1.0, 0.0));
    assert_eq!(effect.shape.core_radius, 0.02);
    assert_eq!(effect.surround.light_gain, 0.0);
    assert_eq!(effect.surround.light_color, [0.85, 0.92, 1.0]);
    assert_eq!(effect.surround.light_height_fraction, 0.5);
    assert_eq!(effect.surround.impact_radius, 0.0);
    assert_eq!(effect.surround.impact_decay, 0.5);
    assert_eq!(effect.surround.thunder_speed, 343.0);
}
