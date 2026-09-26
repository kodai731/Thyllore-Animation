use cgmath::{Quaternion, Vector3};

pub fn water_position_get(effect: &crate::WaterTorusEffect) -> [f32; 3] {
    [effect.position.x, effect.position.y, effect.position.z]
}

pub fn water_position_set(effect: &mut crate::WaterTorusEffect, v: [f32; 3]) {
    effect.position = Vector3::new(v[0], v[1], v[2]);
}

pub fn water_rotation_get(effect: &crate::WaterTorusEffect) -> [f32; 4] {
    [
        effect.rotation.s,
        effect.rotation.v.x,
        effect.rotation.v.y,
        effect.rotation.v.z,
    ]
}

pub fn water_rotation_set(effect: &mut crate::WaterTorusEffect, v: [f32; 4]) {
    effect.rotation = Quaternion::new(v[0], v[1], v[2], v[3]);
}

#[cfg(test)]
mod tests {
    use crate::water::*;
    use thyllore_scene_core::SceneComponent;

    const DEFAULT_JSON: &str = include_str!("scene_format_default.json");
    const WATER_UI_PARAMS_JSON: &str = include_str!("ui_params.json");
    const WATER_PARAMETER_OWNERSHIP_JSON: &str = include_str!("parameter_ownership.json");

    #[test]
    fn test_default_json_matches_fixture() {
        let json = serde_json::to_string(&WaterTorusEffect::default()).expect("serialize");
        assert_eq!(json, DEFAULT_JSON.trim());
    }

    #[test]
    fn test_water_ui_params_match_fixture() {
        let params: Vec<serde_json::Value> = WATER_UI_PARAMS
            .iter()
            .map(|param| {
                serde_json::json!({
                    "name": param.name,
                    "group": param.group,
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
        assert_eq!(current, WATER_UI_PARAMS_JSON.trim());
    }

    #[test]
    fn test_water_parameter_ownership_matches_fixture() {
        let ownership: Vec<serde_json::Value> = WATER_PARAMETER_OWNERSHIP
            .iter()
            .map(|(name, tag)| {
                serde_json::json!({
                    "name": name,
                    "tag": match tag {
                        WaterParameterOwner::Frame => "frame",
                    },
                })
            })
            .collect();
        let current = serde_json::to_string(&ownership).expect("serialize ownership");
        assert_eq!(current, WATER_PARAMETER_OWNERSHIP_JSON.trim());
    }

    #[test]
    fn test_scene_component_reflection_matches_serialized_keys() {
        let value = serde_json::to_value(WaterTorusEffect::default()).expect("serialize");
        let keys: Vec<&str> = value
            .as_object()
            .expect("flat object")
            .keys()
            .map(String::as_str)
            .collect();
        let mut declared = WaterTorusEffect::PERSISTED_FIELDS.to_vec();
        declared.sort_unstable();
        assert_eq!(keys, declared);
        assert_eq!(WaterTorusEffect::TYPE_KEY, "water_torus");
    }
    use thyllore_scene_core::{find_scalar_param, find_ui_param, UiKind};

    #[test]
    fn test_ron_struct_syntax_roundtrip() {
        let mut effect = WaterTorusEffect::default();
        effect.major_radius = 2.5;
        effect.reflect_strength = 0.8;

        let text = ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new())
            .expect("ron serialize");
        let restored: WaterTorusEffect = ron::from_str(&text).expect("ron deserialize");
        assert_eq!(restored.major_radius, 2.5);
        assert_eq!(restored.reflect_strength, 0.8);
    }

    #[test]
    fn test_scalar_param_names_are_unique() {
        let mut names: Vec<&str> = WATER_SCALAR_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
    }

    #[test]
    fn test_scalar_param_set_then_get_reaches_a_fixpoint() {
        for (i, param) in WATER_SCALAR_PARAMS.iter().enumerate() {
            let mut effect = WaterTorusEffect::default();
            (param.set)(&mut effect, 3.0 + i as f32);
            let first = (param.get)(&effect);
            (param.set)(&mut effect, first);
            assert_eq!((param.get)(&effect), first, "{}", param.name);
        }
    }

    #[test]
    fn test_every_ui_param_has_scalar_accessors_and_unique_name() {
        let mut names: Vec<&str> = WATER_UI_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
        for param in WATER_UI_PARAMS {
            for accessor_name in param.scalar_accessor_names() {
                assert!(
                    find_scalar_param(WATER_SCALAR_PARAMS, &accessor_name).is_some(),
                    "{accessor_name}"
                );
            }
            assert!(param.min < param.max, "{}", param.name);
        }
    }

    #[test]
    fn test_ui_param_groups_cover_every_water_group_in_display_order() {
        let mut groups: Vec<&str> = Vec::new();
        for param in WATER_UI_PARAMS {
            if !param.group.is_empty() && !groups.contains(&param.group) {
                groups.push(param.group);
            }
        }
        assert_eq!(
            groups,
            ["shape", "optics", "flow", "wave", "lighting", "look"]
        );
    }

    #[test]
    fn test_ui_param_group_members_match_their_group() {
        let expected: &[(&str, &[&str])] = &[
            ("shape", &["major_radius", "minor_radius"]),
            ("optics", &["ior", "absorption"]),
            ("flow", &["flow_longitudinal", "flow_meridional"]),
            (
                "wave",
                &[
                    "wave_amplitude",
                    "wave_frequency",
                    "wave_speed",
                    "wave_dispersion",
                    "wave_lb_blend",
                ],
            ),
            (
                "lighting",
                &[
                    "light_intensity",
                    "highlight_sharpness",
                    "sky_brightness",
                    "scatter_strength",
                    "scatter_anisotropy",
                ],
            ),
            (
                "look",
                &[
                    "reflect_strength",
                    "refract_strength",
                    "caustic_strength",
                    "tint",
                ],
            ),
        ];

        for (group, members) in expected {
            let declared: Vec<&str> = WATER_UI_PARAMS
                .iter()
                .filter(|param| param.group == *group)
                .map(|param| param.name)
                .collect();
            assert_eq!(declared, *members, "{group}");
            for name in *members {
                let param =
                    find_ui_param(WATER_UI_PARAMS, name).unwrap_or_else(|| panic!("{name}"));
                for accessor_name in param.scalar_accessor_names() {
                    assert!(
                        find_scalar_param(WATER_SCALAR_PARAMS, &accessor_name).is_some(),
                        "{accessor_name}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_color_kinds_are_assigned_to_absorption_and_tint() {
        let kind_of = |name: &str| find_ui_param(WATER_UI_PARAMS, name).map(|p| p.kind);
        assert_eq!(kind_of("absorption"), Some(UiKind::Absorption));
        assert_eq!(kind_of("tint"), Some(UiKind::Color));
        assert_eq!(kind_of("ior"), Some(UiKind::Scalar));
    }

    #[test]
    fn test_absorption_serializes_as_one_vector_field() {
        let value = serde_json::to_value(WaterTorusEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        assert!(object["absorption"].is_array());
        assert!(!object.contains_key("absorption_r"));
    }

    #[test]
    fn test_runtime_ui_params_are_flagged_as_not_persisted() {
        for param in WATER_UI_PARAMS {
            let is_runtime = matches!(param.name, "time" | "time_scale" | "time_offset");
            assert_eq!(param.persisted, !is_runtime, "{}", param.name);
        }
    }

    #[test]
    fn test_overwrite_persisted_fields_keeps_runtime_state() {
        let mut loaded = WaterTorusEffect::default();
        loaded.major_radius = 5.0;
        loaded.time = 5.0;

        let mut target = WaterTorusEffect::default();
        target.time = 2.5;
        target.time_scale = 3.0;

        overwrite_water_persisted_fields(&mut target, &loaded);
        assert_eq!(target.major_radius, 5.0);
        assert_eq!(target.time, 2.5);
        assert_eq!(target.time_scale, 3.0);
    }

    #[test]
    fn test_runtime_params_are_not_serialized() {
        let value = serde_json::to_value(WaterTorusEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        for name in ["time", "time_scale", "time_offset"] {
            assert!(!object.contains_key(name), "{name} must stay runtime-only");
        }
        assert_eq!(object.len(), WATER_PARAMETER_OWNERSHIP.len());
    }

    #[test]
    fn test_primary_params_are_exactly_the_specified_7() {
        let primary: Vec<&str> = WATER_UI_PARAMS
            .iter()
            .filter(|p| p.primary)
            .map(|p| p.name)
            .collect();
        assert_eq!(primary.len(), 7);
        assert!(primary.contains(&"wave_amplitude"));
        assert!(primary.contains(&"wave_frequency"));
        assert!(primary.contains(&"tint"));
        assert!(primary.contains(&"reflect_strength"));
        assert!(primary.contains(&"light_intensity"));
        assert!(primary.contains(&"flow_longitudinal"));
        assert!(primary.contains(&"time_scale"));
    }
}
