use cgmath::{Quaternion, Vector2, Vector3};

pub fn flame_position_get(effect: &crate::FlameEffect) -> [f32; 3] {
    [effect.position.x, effect.position.y, effect.position.z]
}

pub fn flame_position_set(effect: &mut crate::FlameEffect, v: [f32; 3]) {
    effect.position = Vector3::new(v[0], v[1], v[2]);
}

pub fn flame_rotation_get(effect: &crate::FlameEffect) -> [f32; 4] {
    [
        effect.rotation.s,
        effect.rotation.v.x,
        effect.rotation.v.y,
        effect.rotation.v.z,
    ]
}

pub fn flame_rotation_set(effect: &mut crate::FlameEffect, v: [f32; 4]) {
    effect.rotation = Quaternion::new(v[0], v[1], v[2], v[3]);
}

pub fn flame_wind_direction_get(effect: &crate::FlameEffect) -> [f32; 2] {
    [effect.wind.direction.x, effect.wind.direction.y]
}

pub fn flame_wind_direction_set(effect: &mut crate::FlameEffect, v: [f32; 2]) {
    effect.wind.direction = Vector2::new(v[0], v[1]);
}

#[cfg(test)]
mod tests {
    use crate::flame::*;
    use thyllore_scene_core::{SceneComponent, UiKind};

    const DEFAULT_JSON: &str = include_str!("scene_format_default.json");
    const FLAME_UI_PARAMS_JSON: &str = include_str!("ui_params.json");
    const FLAME_PARAMETER_OWNERSHIP_JSON: &str = include_str!("parameter_ownership.json");

    #[test]
    fn test_default_json_matches_fixture() {
        let json = serde_json::to_string(&FlameEffect::default()).expect("serialize");
        assert_eq!(json, DEFAULT_JSON.trim());
    }

    #[test]
    fn test_flame_ui_params_match_fixture() {
        let params: Vec<serde_json::Value> = FLAME_UI_PARAMS
            .iter()
            .map(|param| {
                serde_json::json!({
                    "name": param.name,
                    "group": param.group,
                    "label": param.label,
                    "kind": match param.kind {
                        UiKind::Scalar => "scalar",
                        UiKind::Color => "color",
                        UiKind::Absorption => "absorption",
                        UiKind::Offset => "offset",
                    },
                    "min": param.min,
                    "max": param.max,
                    "format": param.format,
                    "tooltip": param.tooltip,
                    "persisted": param.persisted,
                })
            })
            .collect();
        let current = serde_json::to_string(&params).expect("serialize ui params");
        assert_eq!(current, FLAME_UI_PARAMS_JSON.trim());
    }

    #[test]
    fn test_flame_parameter_ownership_matches_fixture() {
        let ownership: Vec<serde_json::Value> = PARAMETER_OWNERSHIP
            .iter()
            .map(|(name, tag)| {
                serde_json::json!({
                    "name": name,
                    "tag": match tag {
                        ParameterOwner::Frame => "frame",
                        ParameterOwner::Shape => "shape",
                        ParameterOwner::Style => "style",
                    },
                })
            })
            .collect();
        let current = serde_json::to_string(&ownership).expect("serialize ownership");
        assert_eq!(current, FLAME_PARAMETER_OWNERSHIP_JSON.trim());
    }

    #[test]
    fn test_scene_component_reflection_matches_serialized_keys() {
        let value = serde_json::to_value(FlameEffect::default()).expect("serialize");
        let keys: Vec<&str> = value
            .as_object()
            .expect("flat object")
            .keys()
            .map(String::as_str)
            .collect();
        let mut declared = FlameEffect::PERSISTED_FIELDS.to_vec();
        declared.sort_unstable();
        assert_eq!(keys, declared);
        assert_eq!(FlameEffect::TYPE_KEY, "flame");
    }
    use thyllore_scene_core::find_scalar_param;

    #[test]
    fn test_missing_envelope_keys_take_legacy_scene_defaults() {
        let effect: FlameEffect = serde_json::from_str("{}").expect("all keys defaulted");
        assert_eq!(effect.envelope.peak, 0.35);
        assert_eq!(effect.envelope.base, 0.45);
        assert_eq!(effect.envelope.tail, 1.6);
        assert_eq!(effect.height, FlameEffect::default().height);
    }

    #[test]
    fn test_overwrite_persisted_fields_keeps_runtime_state() {
        let mut loaded = FlameEffect::default();
        loaded.height = 9.0;
        loaded.time = 5.0;
        loaded.emitter.kind = 1;

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
    fn test_ron_struct_syntax_roundtrip() {
        let mut effect = FlameEffect::default();
        effect.height = 3.25;
        effect.mix.scale = 0.5;

        let text = ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new())
            .expect("ron serialize");
        let restored: FlameEffect = ron::from_str(&text).expect("ron deserialize");
        assert_eq!(restored.height, 3.25);
        assert_eq!(restored.mix.scale, 0.5);
    }

    #[test]
    fn test_scalar_param_names_are_unique() {
        let mut names: Vec<&str> = FLAME_SCALAR_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
    }

    #[test]
    fn test_scalar_param_set_then_get_reaches_a_fixpoint() {
        for (i, param) in FLAME_SCALAR_PARAMS.iter().enumerate() {
            let mut effect = FlameEffect::default();
            (param.set)(&mut effect, 3.0 + i as f32);
            let first = (param.get)(&effect);
            (param.set)(&mut effect, first);
            assert_eq!((param.get)(&effect), first, "{}", param.name);
        }
    }

    #[test]
    fn test_bool_scalar_param_maps_zero_and_nonzero() {
        let param = find_scalar_param(FLAME_SCALAR_PARAMS, "use_blackbody").expect("registered");
        let mut effect = FlameEffect::default();
        (param.set)(&mut effect, 1.0);
        assert!(effect.color.use_blackbody);
        (param.set)(&mut effect, 0.0);
        assert!(!effect.color.use_blackbody);
    }

    #[test]
    fn test_wind_aliases_write_wind_direction_components() {
        let mut effect = FlameEffect::default();
        (find_scalar_param(FLAME_SCALAR_PARAMS, "wind_x")
            .expect("registered")
            .set)(&mut effect, 0.25);
        (find_scalar_param(FLAME_SCALAR_PARAMS, "wind_z")
            .expect("registered")
            .set)(&mut effect, -0.5);
        assert_eq!(effect.wind.direction.x, 0.25);
        assert_eq!(effect.wind.direction.y, -0.5);
    }

    #[test]
    fn test_every_ui_param_has_a_scalar_accessor_and_unique_name() {
        let mut names: Vec<&str> = FLAME_UI_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
        for param in FLAME_UI_PARAMS {
            for accessor_name in param.scalar_accessor_names() {
                assert!(
                    find_scalar_param(FLAME_SCALAR_PARAMS, &accessor_name).is_some(),
                    "{accessor_name}"
                );
            }
            assert!(param.min < param.max, "{}", param.name);
        }
    }

    #[test]
    fn test_runtime_params_are_not_serialized() {
        let value = serde_json::to_value(FlameEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        for name in ["time", "warp_y_scale", "emitter_kind", "boundary_amp"] {
            assert!(!object.contains_key(name), "{name} must stay runtime-only");
        }
        assert_eq!(object.len(), PARAMETER_OWNERSHIP.len());
    }

    #[test]
    fn test_exactly_eight_primary_params() {
        let primary: Vec<&str> = FLAME_UI_PARAMS
            .iter()
            .filter(|p| p.primary)
            .map(|p| p.name)
            .collect();
        assert_eq!(primary.len(), 8);
        assert!(primary.contains(&"radius"));
        assert!(primary.contains(&"height"));
        assert!(primary.contains(&"noise_amplitude"));
        assert!(primary.contains(&"noise_contrast"));
        assert!(primary.contains(&"color_base"));
        assert!(primary.contains(&"color_tip"));
        assert!(primary.contains(&"intensity"));
        assert!(primary.contains(&"time_scale"));
    }
}
