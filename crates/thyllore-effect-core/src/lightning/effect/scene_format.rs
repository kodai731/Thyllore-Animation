use crate::lightning::*;
use cgmath::{Quaternion, Vector3};
use thyllore_scene_core::declare_scene_format;

declare_scene_format! {
    component: LightningEffect,
    record: LightningSceneRecord,
    items {
        key: "lightning",
        snapshot: lightning_parameter_snapshot,
        scalars: LIGHTNING_SCALAR_PARAMS,
        ui: LIGHTNING_UI_PARAMS,
        overwrite: overwrite_lightning_persisted_fields,
    },
    persisted {
        position: [f32; 3] {
            get: |e| [e.position.x, e.position.y, e.position.z],
            set: |e, v| e.position = Vector3::new(v[0], v[1], v[2]),
        },
        rotation: [f32; 4] {
            get: |e| [e.rotation.s, e.rotation.v.x, e.rotation.v.y, e.rotation.v.z],
            set: |e, v| e.rotation = Quaternion::new(v[0], v[1], v[2], v[3]),
        },
        source: LightningSource {
            get: |e| e.shape.source.clone(),
            set: |e, v| e.shape.source = v,
        },
        end_offset: [f32; 3] {
            get: |e| e.shape.end_offset,
            set: |e, v| e.shape.end_offset = v,
            scalars {
                end_offset_x: {
                    get: |e| e.shape.end_offset[0],
                    set: |e, v| e.shape.end_offset[0] = v,
                },
                end_offset_y: {
                    get: |e| e.shape.end_offset[1],
                    set: |e, v| e.shape.end_offset[1] = v,
                },
                end_offset_z: {
                    get: |e| e.shape.end_offset[2],
                    set: |e, v| e.shape.end_offset[2] = v,
                },
            },
            ui {
                kind: Offset,
                min: -50.0,
                max: 50.0,
                format: "%.2f",
                tooltip: "End point of the main strike relative to the effect origin",
                group: "shape",
            },
        },
        strikes_per_burst: u32 {
            get: |e| e.shape.strikes_per_burst,
            set: |e, v| e.shape.strikes_per_burst = v,
            ui {
                min: 1.0,
                max: 32.0,
                format: "%.0f",
                group: "shape",
            },
        },
        detail_levels: u32 {
            get: |e| e.shape.detail_levels,
            set: |e, v| e.shape.detail_levels = v,
            ui {
                min: 1.0,
                max: 8.0,
                format: "%.0f",
                tooltip: "Midpoint displacement subdivisions of the channel; the segment count is 2^detail_levels",
                group: "shape",
            },
        },
        tortuosity: f32 {
            get: |e| e.shape.tortuosity,
            set: |e, v| e.shape.tortuosity = v,
            ui {
                primary,
                min: 0.0,
                max: 2.0,
                format: "%.2f",
                tooltip: "Lateral displacement of the first subdivision relative to the strike length",
                group: "shape",
            },
        },
        roughness: f32 {
            get: |e| e.shape.roughness,
            set: |e, v| e.shape.roughness = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Decay of the displacement per subdivision level; 0.5 is a Brownian channel",
                group: "shape",
            },
        },
        core_radius: f32 {
            get: |e| e.shape.core_radius,
            set: |e, v| e.shape.core_radius = v,
            ui {
                primary,
                min: 0.001,
                max: 1.0,
                format: "%.3f",
                group: "shape",
            },
        },
        tip_radius_ratio: f32 {
            get: |e| e.shape.tip_radius_ratio,
            set: |e, v| e.shape.tip_radius_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Core radius at the strike tip as a fraction of core_radius",
                group: "shape",
            },
        },
        edge_fraction: f32 {
            get: |e| e.shape.edge_fraction,
            set: |e, v| e.shape.edge_fraction = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Fraction of the core radius over which the coverage falls to zero",
                group: "shape",
            },
        },
        branch_depth: u32 {
            get: |e| e.branch.depth,
            set: |e, v| e.branch.depth = v,
            ui {
                min: 0.0,
                max: 5.0,
                format: "%.0f",
                tooltip: "Recursion depth of the branch tree; 0 leaves the main channel alone",
                group: "branch",
            },
        },
        branch_probability: f32 {
            get: |e| e.branch.probability,
            set: |e, v| e.branch.probability = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        branch_count: f32 {
            get: |e| e.branch.count,
            set: |e, v| e.branch.count = v,
            ui {
                primary,
                min: 0.0,
                max: 32.0,
                format: "%.2f",
                tooltip: "Number of branches leaving the main channel; the fraction fades the last one",
                group: "branch",
            },
        },
        branch_zone_start: f32 {
            get: |e| e.branch.zone_start,
            set: |e, v| e.branch.zone_start = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Where along the main channel (0 = start, 1 = end) branches begin",
                group: "branch",
            },
        },
        branch_zone_end: f32 {
            get: |e| e.branch.zone_end,
            set: |e, v| e.branch.zone_end = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Where along the main channel (0 = start, 1 = end) branches stop",
                group: "branch",
            },
        },
        branch_angle: f32 {
            get: |e| e.branch.angle,
            set: |e, v| e.branch.angle = v,
            ui {
                primary,
                min: 0.0,
                max: 1.57,
                format: "%.2f",
                tooltip: "Radians between a branch and its parent channel",
                group: "branch",
            },
        },
        branch_length_ratio: f32 {
            get: |e| e.branch.length_ratio,
            set: |e, v| e.branch.length_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        branch_radius_ratio: f32 {
            get: |e| e.branch.radius_ratio,
            set: |e, v| e.branch.radius_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        branch_intensity_ratio: f32 {
            get: |e| e.branch.intensity_ratio,
            set: |e, v| e.branch.intensity_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        core_intensity: f32 {
            get: |e| e.look.core_intensity,
            set: |e, v| e.look.core_intensity = v,
            ui {
                primary,
                min: 0.0,
                max: 200.0,
                format: "%.1f",
                tooltip: "Radiance of the saturated channel core",
                group: "look",
            },
        },
        core_color: [f32; 3] {
            get: |e| e.look.core_color,
            set: |e, v| e.look.core_color = v,
            scalars: rgb,
            ui {
                kind: Color,
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "look",
            },
        },
        rim_ratio: f32 {
            get: |e| e.look.rim_ratio,
            set: |e, v| e.look.rim_ratio = v,
            ui {
                min: 1.0,
                max: 20.0,
                format: "%.2f",
                tooltip: "Radius of the rim sheath as a multiple of the core radius",
                group: "look",
            },
        },
        rim_intensity: f32 {
            get: |e| e.look.rim_intensity,
            set: |e, v| e.look.rim_intensity = v,
            ui {
                min: 0.0,
                max: 100.0,
                format: "%.2f",
                group: "look",
            },
        },
        rim_color: [f32; 3] {
            get: |e| e.look.rim_color,
            set: |e, v| e.look.rim_color = v,
            scalars: rgb,
            ui {
                primary,
                kind: Color,
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "look",
            },
        },
        beam_radius: f32 {
            get: |e| e.look.beam_radius,
            set: |e, v| e.look.beam_radius = v,
            ui {
                min: 0.0,
                max: 10.0,
                format: "%.2f",
                tooltip: "Radius of the beam the arcs wrap around; 0 keeps a bare channel",
                group: "look",
            },
        },
        beam_arc_count: u32 {
            get: |e| e.look.beam_arc_count,
            set: |e, v| e.look.beam_arc_count = v,
            ui {
                min: 0.0,
                max: 64.0,
                format: "%.0f",
                group: "look",
            },
        },
        flash_gain: f32 {
            get: |e| e.look.flash_gain,
            set: |e, v| e.look.flash_gain = v,
            ui {
                min: 0.0,
                max: 10.0,
                format: "%.2f",
                tooltip: "Radiance of the ambient flash emitted while a stroke is alive; 0 disables it",
                group: "look",
            },
        },
        flash_radius: f32 {
            get: |e| e.look.flash_radius,
            set: |e, v| e.look.flash_radius = v,
            ui {
                min: 0.0,
                max: 100.0,
                format: "%.2f",
                group: "look",
            },
        },
        end_variance: f32 {
            get: |e| e.shape.end_variance,
            set: |e, v| e.shape.end_variance = v,
            ui {
                min: 0.0,
                max: 4.0,
                format: "%.2f",
                tooltip: "Radius each discharge scatters its end point by, across the bolt's direction",
                group: "shape",
            },
        },
        growth_time: f32 {
            get: |e| e.timing.growth_time,
            set: |e, v| e.timing.growth_time = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.3f",
                tooltip: "Seconds the discharge takes to reach its end; 0 draws it whole at once",
                group: "timing",
            },
        },
        burst_start: f32 {
            get: |e| e.timing.burst_start,
            set: |e, v| e.timing.burst_start = v,
            ui {
                min: 0.0,
                max: 10.0,
                format: "%.2f",
                group: "timing",
            },
        },
        burst_interval: f32 {
            get: |e| e.timing.burst_interval,
            set: |e, v| e.timing.burst_interval = v,
            ui {
                primary,
                min: 0.01,
                max: 10.0,
                format: "%.2f",
                group: "timing",
            },
        },
        burst_jitter: f32 {
            get: |e| e.timing.burst_jitter,
            set: |e, v| e.timing.burst_jitter = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Fraction of burst_interval the burst start is randomly shifted by",
                group: "timing",
            },
        },
        burst_count: u32 {
            get: |e| e.timing.burst_count,
            set: |e, v| e.timing.burst_count = v,
            ui {
                min: 0.0,
                max: 16.0,
                format: "%.0f",
                tooltip: "Number of bursts to play; 0 repeats forever",
                group: "timing",
            },
        },
        attack_time: f32 {
            get: |e| e.timing.attack_time,
            set: |e, v| e.timing.attack_time = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        sustain_time: f32 {
            get: |e| e.timing.sustain_time,
            set: |e, v| e.timing.sustain_time = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        release_time: f32 {
            get: |e| e.timing.release_time,
            set: |e, v| e.timing.release_time = v,
            ui {
                min: 0.0,
                max: 2.0,
                format: "%.3f",
                group: "timing",
            },
        },
        stroke_count: u32 {
            get: |e| e.timing.stroke_count,
            set: |e, v| e.timing.stroke_count = v,
            ui {
                min: 1.0,
                max: 8.0,
                format: "%.0f",
                tooltip: "Return strokes per burst, played stroke_interval apart",
                group: "timing",
            },
        },
        stroke_interval: f32 {
            get: |e| e.timing.stroke_interval,
            set: |e, v| e.timing.stroke_interval = v,
            ui {
                min: 0.001,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        stroke_decay: f32 {
            get: |e| e.timing.stroke_decay,
            set: |e, v| e.timing.stroke_decay = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Intensity of each return stroke relative to the previous one",
                group: "timing",
            },
        },
        flicker_amplitude: f32 {
            get: |e| e.timing.flicker_amplitude,
            set: |e, v| e.timing.flicker_amplitude = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "timing",
            },
        },
        flicker_period: f32 {
            get: |e| e.timing.flicker_period,
            set: |e, v| e.timing.flicker_period = v,
            ui {
                min: 0.001,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        reseed_level: u32 {
            get: |e| e.timing.reseed_level,
            set: |e, v| e.timing.reseed_level = v,
            ui {
                min: 0.0,
                max: 8.0,
                format: "%.0f",
                tooltip: "Subdivision level above which the channel is reshaped every reseed_period",
                group: "timing",
            },
        },
        reseed_period: f32 {
            get: |e| e.timing.reseed_period,
            set: |e, v| e.timing.reseed_period = v,
            ui {
                min: 0.01,
                max: 10.0,
                format: "%.2f",
                group: "timing",
            },
        },
        charge_ramp: f32 {
            get: |e| e.timing.charge_ramp,
            set: |e, v| e.timing.charge_ramp = v,
            ui {
                min: 0.0,
                max: 2.0,
                format: "%.2f",
                tooltip: "Seconds of dim leader glow before the first return stroke; 0 strikes instantly",
                group: "timing",
            },
        },
        seed: u32 {
            get: |e| e.timing.seed,
            set: |e, v| e.timing.seed = v,
            ui {
                min: 0.0,
                max: 9999.0,
                format: "%.0f",
                tooltip: "Seed of the deterministic hash that shapes the channel and the burst jitter",
                group: "timing",
            },
        },
    },
    runtime {
        time: f32 {
            get: |e| e.time,
            set: |e, v| e.time = v,
            ui {
                min: 0.0,
                max: 100.0,
                format: "%.2f",
            },
        },
        time_scale: f32 {
            get: |e| e.time_scale,
            set: |e, v| e.time_scale = v,
            ui {
                primary,
                min: 0.0,
                max: 4.0,
                format: "%.2f",
            },
        },
        time_offset: f32 {
            get: |e| e.time_offset,
            set: |e, v| e.time_offset = v,
            ui {
                min: -100.0,
                max: 100.0,
                format: "%.2f",
            },
        },
    },
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
