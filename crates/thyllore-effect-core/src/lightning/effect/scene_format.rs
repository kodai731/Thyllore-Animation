use crate::lightning::*;
use cgmath::{Quaternion, Vector3};

pub fn lightning_position_get(effect: &LightningEffect) -> [f32; 3] {
    [effect.position.x, effect.position.y, effect.position.z]
}

pub fn lightning_position_set(effect: &mut LightningEffect, v: [f32; 3]) {
    effect.position = Vector3::new(v[0], v[1], v[2]);
}

pub fn lightning_rotation_get(effect: &LightningEffect) -> [f32; 4] {
    [
        effect.rotation.s,
        effect.rotation.v.x,
        effect.rotation.v.y,
        effect.rotation.v.z,
    ]
}

pub fn lightning_rotation_set(effect: &mut LightningEffect, v: [f32; 4]) {
    effect.rotation = Quaternion::new(v[0], v[1], v[2], v[3]);
}

pub fn lightning_source_get(effect: &LightningEffect) -> LightningSource {
    effect.shape.source.clone()
}

pub fn lightning_source_set(effect: &mut LightningEffect, v: LightningSource) {
    effect.shape.source = v;
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_scene_core::{find_scalar_param, find_ui_param, SceneComponent, UiKind};

    const DEFAULT_JSON: &str = include_str!("scene_format_default.json");
    const LIGHTNING_UI_PARAMS_JSON: &str = include_str!("ui_params.json");

    #[test]
    fn test_default_json_matches_fixture() {
        let json = serde_json::to_string(&LightningEffect::default()).expect("serialize");
        assert_eq!(json, DEFAULT_JSON.trim());
    }

    #[test]
    fn test_lightning_ui_params_match_fixture() {
        let params: Vec<serde_json::Value> = LIGHTNING_UI_PARAMS
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
                    "primary": param.primary,
                    "label": param.label,
                })
            })
            .collect();
        let current = serde_json::to_string(&params).expect("serialize ui params");
        assert_eq!(current, LIGHTNING_UI_PARAMS_JSON.trim());
    }

    #[test]
    fn test_scene_component_reflection_matches_serialized_keys() {
        let value = serde_json::to_value(LightningEffect::default()).expect("serialize");
        let keys: Vec<&str> = value
            .as_object()
            .expect("flat object")
            .keys()
            .map(String::as_str)
            .collect();
        let mut declared = LightningEffect::PERSISTED_FIELDS.to_vec();
        declared.sort_unstable();
        assert_eq!(keys, declared);
        assert_eq!(LightningEffect::TYPE_KEY, "lightning");
    }

    #[test]
    fn test_ron_roundtrip_keeps_the_source_enum() {
        let mut effect = LightningEffect::default();
        effect.shape.source = LightningSource::Shell { radius: 2.5 };
        effect.shape.core_radius = 0.12;

        let text = ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new())
            .expect("ron serialize");
        let restored: LightningEffect = ron::from_str(&text).expect("ron deserialize");
        assert_eq!(
            restored.shape.source,
            LightningSource::Shell { radius: 2.5 }
        );
        assert_eq!(restored.shape.core_radius, 0.12);
    }

    #[test]
    fn test_every_ui_param_has_scalar_accessors_and_unique_name() {
        let mut names: Vec<&str> = LIGHTNING_UI_PARAMS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
        for param in LIGHTNING_UI_PARAMS {
            for accessor_name in param.scalar_accessor_names() {
                assert!(
                    find_scalar_param(LIGHTNING_SCALAR_PARAMS, &accessor_name).is_some(),
                    "{accessor_name}"
                );
            }
            assert!(param.min < param.max, "{}", param.name);
        }
    }

    #[test]
    fn test_ui_param_groups_cover_every_lightning_group_in_display_order() {
        let mut groups: Vec<&str> = Vec::new();
        for param in LIGHTNING_UI_PARAMS {
            if !param.group.is_empty() && !groups.contains(&param.group) {
                groups.push(param.group);
            }
        }
        assert_eq!(groups, ["shape", "branch", "look", "timing"]);
    }

    #[test]
    fn test_colors_are_colors_and_serialize_as_one_vector() {
        for name in ["core_color", "rim_color"] {
            assert_eq!(
                find_ui_param(LIGHTNING_UI_PARAMS, name).map(|p| p.kind),
                Some(UiKind::Color),
                "{name}"
            );
        }
        let value = serde_json::to_value(LightningEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        assert!(object["core_color"].is_array());
        assert!(!object.contains_key("core_color_r"));
    }

    #[test]
    fn test_runtime_params_are_not_serialized() {
        let value = serde_json::to_value(LightningEffect::default()).expect("serialize");
        let object = value.as_object().expect("flat object");
        for name in ["time", "time_scale", "time_offset"] {
            assert!(!object.contains_key(name), "{name} must stay runtime-only");
        }
    }

    #[test]
    fn test_overwrite_persisted_fields_keeps_runtime_state() {
        let mut loaded = LightningEffect::default();
        loaded.shape.core_radius = 0.2;
        loaded.time = 5.0;

        let mut target = LightningEffect::default();
        target.time = 2.5;

        overwrite_lightning_persisted_fields(&mut target, &loaded);
        assert_eq!(target.shape.core_radius, 0.2);
        assert_eq!(target.time, 2.5);
    }

    #[test]
    fn test_exactly_eight_primary_params() {
        let primary: Vec<&str> = LIGHTNING_UI_PARAMS
            .iter()
            .filter(|p| p.primary)
            .map(|p| p.name)
            .collect();
        assert_eq!(primary.len(), 8);
        assert!(primary.contains(&"core_radius"));
        assert!(primary.contains(&"tortuosity"));
        assert!(primary.contains(&"branch_count"));
        assert!(primary.contains(&"branch_angle"));
        assert!(primary.contains(&"core_intensity"));
        assert!(primary.contains(&"rim_color"));
        assert!(primary.contains(&"burst_interval"));
        assert!(primary.contains(&"time_scale"));
    }
}
