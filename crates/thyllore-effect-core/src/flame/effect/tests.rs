use crate::flame::*;
use crate::test_support::scene::*;
use thyllore_scene_core::find_scalar_param;

#[test]
fn test_tables_are_consistent() {
    assert_tables_are_consistent::<FlameEffect>(
        &FLAME_SCALAR_PARAMS,
        &FLAME_UI_PARAMS,
        PARAMETER_OWNERSHIP.len(),
    );
    assert_eq!(
        <FlameEffect as thyllore_scene_core::SceneComponent>::TYPE_KEY,
        "flame"
    );
}

#[test]
fn test_codes_lie_in_the_flame_block() {
    assert_codes_lie_in_block(&FLAME_SCALAR_PARAMS, FLAME_SCALAR_CODES);
}

#[test]
fn test_ron_struct_syntax_roundtrip() {
    let mut effect = FlameEffect::default();
    effect.height = 3.25;
    effect.mix.scale = 0.5;

    let text =
        ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new()).expect("ron serialize");
    let restored: FlameEffect = ron::from_str(&text).expect("ron deserialize");
    assert_eq!(restored.height, 3.25);
    assert_eq!(restored.mix.scale, 0.5);
}

#[test]
fn test_overwrite_persisted_fields_keeps_runtime_state() {
    let mut loaded = FlameEffect::default();
    loaded.height = 9.0;
    loaded.time = 5.0;
    loaded.emitter.kind = 1;
    loaded.warp.y_scale = 0.4;

    let mut target = FlameEffect::default();
    target.time = 2.5;
    target.emitter.kind = 2;
    target.warp.y_scale = 0.9;

    overwrite_persisted_fields(&mut target, &loaded);
    assert_eq!(target.height, 9.0);
    assert_eq!(target.time, 2.5);
    assert_eq!(target.emitter.kind, 2);
    assert_eq!(target.warp.y_scale, 0.9);
}

#[test]
fn test_bool_scalar_param_maps_zero_and_nonzero() {
    let param = find_scalar_param(&FLAME_SCALAR_PARAMS, "color_use_blackbody").expect("param");
    let mut effect = FlameEffect::default();
    (param.set)(&mut effect, 1.0);
    assert!(effect.color.use_blackbody);
    (param.set)(&mut effect, 0.0);
    assert!(!effect.color.use_blackbody);
}

#[test]
fn test_wind_aliases_write_wind_direction_components() {
    let mut effect = FlameEffect::default();
    let set = |effect: &mut FlameEffect, name: &str, value: f32| {
        (find_scalar_param(&FLAME_SCALAR_PARAMS, name)
            .expect(name)
            .set)(effect, value)
    };
    set(&mut effect, "wind_direction_x", 0.25);
    set(&mut effect, "wind_direction_y", -0.5);
    assert_eq!(effect.wind.direction.x, 0.25);
    assert_eq!(effect.wind.direction.y, -0.5);
}

#[test]
fn test_exactly_eight_primary_params() {
    let mut primary = primary_names(&FLAME_UI_PARAMS);
    primary.sort_unstable();
    assert_eq!(
        primary,
        [
            "color_base",
            "color_tip",
            "height",
            "intensity",
            "noise_amplitude",
            "noise_contrast",
            "radius",
            "time_scale",
        ]
    );
}
