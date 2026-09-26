use cgmath::{Quaternion, Vector3};

pub fn wind_position_get(effect: &crate::WindTornadoEffect) -> [f32; 3] {
    [effect.position.x, effect.position.y, effect.position.z]
}

pub fn wind_position_set(effect: &mut crate::WindTornadoEffect, v: [f32; 3]) {
    effect.position = Vector3::new(v[0], v[1], v[2]);
}

pub fn wind_rotation_get(effect: &crate::WindTornadoEffect) -> [f32; 4] {
    [
        effect.rotation.s,
        effect.rotation.v.x,
        effect.rotation.v.y,
        effect.rotation.v.z,
    ]
}

pub fn wind_rotation_set(effect: &mut crate::WindTornadoEffect, v: [f32; 4]) {
    effect.rotation = Quaternion::new(v[0], v[1], v[2], v[3]);
}

#[cfg(test)]
mod tests {
    use crate::wind::*;
    use thyllore_scene_core::SceneComponent;

    const DEFAULT_JSON: &str = include_str!("scene_format_default.json");
    const WIND_UI_PARAMS_JSON: &str = include_str!("ui_params.json");
    const WIND_PARAMETER_OWNERSHIP_JSON: &str = include_str!("parameter_ownership.json");

    #[test]
    fn test_default_json_matches_fixture() {
        let json = serde_json::to_string(&WindTornadoEffect::default()).expect("serialize");
        assert_eq!(json, DEFAULT_JSON.trim());
    }

    #[test]
    fn test_wind_ui_params_match_fixture() {
        let params: Vec<serde_json::Value> = WIND_UI_PARAMS
            .iter()
            .map(|param| {
                serde_json::json!({
                    "name": param.name,
                    "group": param.group,
                    "kind": match param.kind {
                        thyllore_scene_core::UiKind::Scalar => "scalar",
                        thyllore_scene_core::UiKind::Color => "color",
                        thyllore_scene_core::UiKind::Absorption => "absorption",
                        thyllore_scene_core::UiKind::Offset => "offset",
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
        assert_eq!(current, WIND_UI_PARAMS_JSON.trim());
    }

    #[test]
    fn test_wind_parameter_ownership_matches_fixture() {
        let ownership: Vec<serde_json::Value> = WIND_PARAMETER_OWNERSHIP
            .iter()
            .map(|(name, tag)| {
                serde_json::json!({
                    "name": name,
                    "tag": match tag {
                        WindParameterOwner::Frame => "frame",
                    },
                })
            })
            .collect();
        let current = serde_json::to_string(&ownership).expect("serialize ownership");
        assert_eq!(current, WIND_PARAMETER_OWNERSHIP_JSON.trim());
    }

    #[test]
    fn test_scene_component_reflection_matches_serialized_keys() {
        let value = serde_json::to_value(WindTornadoEffect::default()).expect("serialize");
        let keys: Vec<&str> = value
            .as_object()
            .expect("flat object")
            .keys()
            .map(String::as_str)
            .collect();
        let mut declared = WindTornadoEffect::PERSISTED_FIELDS.to_vec();
        declared.sort_unstable();
        assert_eq!(keys, declared);
        assert_eq!(WindTornadoEffect::TYPE_KEY, "wind_tornado");
    }
    use thyllore_scene_core::{find_scalar_param, find_ui_param, UiKind};

    #[test]
    fn test_ron_struct_syntax_roundtrip() {
        let mut effect = WindTornadoEffect::default();
        effect.column_height = 3.5;
        effect.wall_width_q = 0.2;

        let text = ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new())
            .expect("ron serialize");
        let restored: WindTornadoEffect = ron::from_str(&text).expect("ron deserialize");
        assert_eq!(restored.column_height, 3.5);
        assert_eq!(restored.wall_width_q, 0.2);
    }

    #[test]
    fn test_scalar_param_names_are_unique() {
        let mut names: Vec<&str> = WIND_SCALAR_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
    }

    #[test]
    fn test_every_ui_param_has_scalar_accessors_and_unique_name() {
        let mut names: Vec<&str> = WIND_UI_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
        for param in WIND_UI_PARAMS {
            for accessor_name in param.scalar_accessor_names() {
                assert!(
                    find_scalar_param(WIND_SCALAR_PARAMS, &accessor_name).is_some(),
                    "{accessor_name}"
                );
            }
            assert!(param.min < param.max, "{}", param.name);
        }
    }

    #[test]
    fn test_ui_param_groups_cover_every_wind_group_in_display_order() {
        let mut groups: Vec<&str> = Vec::new();
        for param in WIND_UI_PARAMS {
            if !param.group.is_empty() && !groups.contains(&param.group) {
                groups.push(param.group);
            }
        }
        assert_eq!(groups, ["shape", "density", "motion", "eddy", "look"]);
    }

    #[test]
    fn test_ui_param_group_members_match_their_group() {
        let expected: &[(&str, &[&str])] = &[
            (
                "shape",
                &[
                    "column_height",
                    "wall_radius_base",
                    "wall_radius_top",
                    "wall_width_q",
                    "top_fade",
                ],
            ),
            ("density", &["density", "wall_strength"]),
            (
                "motion",
                &[
                    "rise_initial_height",
                    "rise_duration",
                    "spread_start",
                    "spread_rate",
                    "dissipate_start",
                    "dissipate_time",
                    "circulation",
                    "streak_order",
                    "streak_twist",
                    "streak_rise_speed",
                    "streak_amplitude",
                ],
            ),
            (
                "eddy",
                &[
                    "eddy_amplitude",
                    "eddy_cell_theta",
                    "eddy_cell_height",
                    "eddy_cell_radial",
                    "eddy_shear",
                    "eddy_speed_spread",
                    "eddy_rise_speed",
                    "eddy_reseed_period",
                    "eddy_erosion",
                    "puff_count_theta",
                    "puff_count_height",
                    "puff_radius",
                    "puff_radius_jitter",
                    "puff_offset_q",
                    "puff_strength",
                    "puff_rise_speed",
                ],
            ),
            (
                "look",
                &["albedo", "ambient_brightness", "phase_g", "sun_intensity"],
            ),
        ];

        let grouped: usize = expected.iter().map(|(_, members)| members.len()).sum();
        let persisted = WIND_UI_PARAMS
            .iter()
            .filter(|param| param.persisted)
            .count();
        assert_eq!(grouped, persisted);

        for (group, members) in expected {
            let declared: Vec<&str> = WIND_UI_PARAMS
                .iter()
                .filter(|param| param.group == *group)
                .map(|param| param.name)
                .collect();
            assert_eq!(declared, *members, "{group}");
        }
    }

    #[test]
    fn test_albedo_is_a_color_and_serializes_as_one_vector() {
        assert_eq!(
            find_ui_param(WIND_UI_PARAMS, "albedo").map(|p| p.kind),
            Some(UiKind::Color)
        );
        let value = serde_json::to_value(WindTornadoEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        assert!(object["albedo"].is_array());
        assert!(!object.contains_key("albedo_r"));
    }

    #[test]
    fn test_runtime_params_are_not_serialized() {
        let value = serde_json::to_value(WindTornadoEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        for name in ["time", "time_scale", "time_offset"] {
            assert!(!object.contains_key(name), "{name} must stay runtime-only");
        }
        assert_eq!(object.len(), WIND_PARAMETER_OWNERSHIP.len());
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
        let primary: Vec<&str> = WIND_UI_PARAMS
            .iter()
            .filter(|p| p.primary)
            .map(|p| p.name)
            .collect();
        assert_eq!(primary.len(), 8);
        assert!(primary.contains(&"column_height"));
        assert!(primary.contains(&"wall_radius_base"));
        assert!(primary.contains(&"circulation"));
        assert!(primary.contains(&"rise_duration"));
        assert!(primary.contains(&"eddy_amplitude"));
        assert!(primary.contains(&"density"));
        assert!(primary.contains(&"albedo"));
        assert!(primary.contains(&"time_scale"));
    }
}
