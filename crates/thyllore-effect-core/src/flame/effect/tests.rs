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
    loaded.emitter.kind = FlameEmitterKind::Ring;
    loaded.warp.y_scale = 0.4;

    let mut target = FlameEffect::default();
    target.time = 2.5;
    target.emitter.kind = FlameEmitterKind::MeshSdf;
    target.warp.y_scale = 0.9;

    overwrite_persisted_fields(&mut target, &loaded);
    assert_eq!(target.height, 9.0);
    assert_eq!(target.time, 2.5);
    assert_eq!(target.emitter.kind, FlameEmitterKind::MeshSdf);
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
fn test_primary_params_are_the_eight_core_knobs_and_the_ring_pair() {
    let mut primary = primary_names(&FLAME_UI_PARAMS);
    primary.sort_unstable();
    assert_eq!(
        primary,
        [
            "color_base",
            "color_tip",
            "emitter_ring_angular_speed",
            "emitter_ring_major_radius",
            "height",
            "intensity",
            "noise_amplitude",
            "noise_contrast",
            "radius",
            "time_scale",
        ]
    );
}

#[test]
fn test_emitter_kind_round_trips_through_its_shader_value() {
    for kind in FlameEmitterKind::ALL {
        assert_eq!(
            FlameEmitterKind::from_shader_value(kind.as_shader_value()),
            Some(kind)
        );
    }
    assert_eq!(
        FlameEmitterKind::from_shader_value(FlameEmitterKind::ALL.len() as u32),
        None
    );
}

#[test]
fn test_ring_params_are_hidden_unless_the_emitter_is_a_ring() {
    let hidden_names = |kind| {
        let emitter = FlameEmitter {
            kind,
            ..FlameEmitter::default()
        };
        let active = emitter_ui_conditions(&emitter);
        FLAME_UI_PARAMS
            .iter()
            .filter(|param| param.is_hidden_in(&active))
            .map(|param| param.name)
            .collect::<Vec<_>>()
    };

    assert!(hidden_names(FlameEmitterKind::Ring).is_empty());
    for kind in [FlameEmitterKind::Cylinder, FlameEmitterKind::MeshSdf] {
        assert_eq!(
            hidden_names(kind),
            ["emitter_ring_major_radius", "emitter_ring_angular_speed"]
        );
    }
}
