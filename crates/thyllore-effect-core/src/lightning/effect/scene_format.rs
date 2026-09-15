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
            get: |e| e.source.clone(),
            set: |e, v| e.source = v,
        },
        end_offset: [f32; 3] {
            get: |e| e.end_offset,
            set: |e, v| e.end_offset = v,
            scalars {
                end_offset_x: {
                    get: |e| e.end_offset[0],
                    set: |e, v| e.end_offset[0] = v,
                },
                end_offset_y: {
                    get: |e| e.end_offset[1],
                    set: |e, v| e.end_offset[1] = v,
                },
                end_offset_z: {
                    get: |e| e.end_offset[2],
                    set: |e, v| e.end_offset[2] = v,
                },
            },
        },
        strikes_per_burst: u32 {
            get: |e| e.strikes_per_burst,
            set: |e, v| e.strikes_per_burst = v,
            ui {
                min: 1.0,
                max: 32.0,
                format: "%.0f",
                group: "shape",
            },
        },
        detail_levels: u32 {
            get: |e| e.detail_levels,
            set: |e, v| e.detail_levels = v,
            ui {
                min: 1.0,
                max: 8.0,
                format: "%.0f",
                tooltip: "Midpoint displacement subdivisions of the channel; the segment count is 2^detail_levels",
                group: "shape",
            },
        },
        tortuosity: f32 {
            get: |e| e.tortuosity,
            set: |e, v| e.tortuosity = v,
            ui {
                min: 0.0,
                max: 2.0,
                format: "%.2f",
                tooltip: "Lateral displacement of the first subdivision relative to the strike length",
                group: "shape",
            },
        },
        roughness: f32 {
            get: |e| e.roughness,
            set: |e, v| e.roughness = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Decay of the displacement per subdivision level; 0.5 is a Brownian channel",
                group: "shape",
            },
        },
        core_radius: f32 {
            get: |e| e.core_radius,
            set: |e, v| e.core_radius = v,
            ui {
                min: 0.001,
                max: 1.0,
                format: "%.3f",
                group: "shape",
            },
        },
        tip_radius_ratio: f32 {
            get: |e| e.tip_radius_ratio,
            set: |e, v| e.tip_radius_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Core radius at the strike tip as a fraction of core_radius",
                group: "shape",
            },
        },
        edge_fraction: f32 {
            get: |e| e.edge_fraction,
            set: |e, v| e.edge_fraction = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Fraction of the core radius over which the coverage falls to zero",
                group: "shape",
            },
        },
        branch_depth: u32 {
            get: |e| e.branch_depth,
            set: |e, v| e.branch_depth = v,
            ui {
                min: 0.0,
                max: 5.0,
                format: "%.0f",
                tooltip: "Recursion depth of the branch tree; 0 leaves the main channel alone",
                group: "branch",
            },
        },
        branch_probability: f32 {
            get: |e| e.branch_probability,
            set: |e, v| e.branch_probability = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        branch_angle: f32 {
            get: |e| e.branch_angle,
            set: |e, v| e.branch_angle = v,
            ui {
                min: 0.0,
                max: 1.57,
                format: "%.2f",
                tooltip: "Radians between a branch and its parent channel",
                group: "branch",
            },
        },
        branch_length_ratio: f32 {
            get: |e| e.branch_length_ratio,
            set: |e, v| e.branch_length_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        branch_radius_ratio: f32 {
            get: |e| e.branch_radius_ratio,
            set: |e, v| e.branch_radius_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        branch_intensity_ratio: f32 {
            get: |e| e.branch_intensity_ratio,
            set: |e, v| e.branch_intensity_ratio = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "branch",
            },
        },
        core_intensity: f32 {
            get: |e| e.core_intensity,
            set: |e, v| e.core_intensity = v,
            ui {
                min: 0.0,
                max: 200.0,
                format: "%.1f",
                tooltip: "Radiance of the saturated channel core",
                group: "look",
            },
        },
        core_color: [f32; 3] {
            get: |e| e.core_color,
            set: |e, v| e.core_color = v,
            scalars: rgb,
            ui {
                kind: Color,
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "look",
            },
        },
        glow_ratio: f32 {
            get: |e| e.glow_ratio,
            set: |e, v| e.glow_ratio = v,
            ui {
                min: 1.0,
                max: 20.0,
                format: "%.2f",
                tooltip: "Radius of the glow sheath as a multiple of the core radius",
                group: "look",
            },
        },
        glow_intensity: f32 {
            get: |e| e.glow_intensity,
            set: |e, v| e.glow_intensity = v,
            ui {
                min: 0.0,
                max: 100.0,
                format: "%.2f",
                group: "look",
            },
        },
        glow_color: [f32; 3] {
            get: |e| e.glow_color,
            set: |e, v| e.glow_color = v,
            scalars: rgb,
            ui {
                kind: Color,
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "look",
            },
        },
        beam_radius: f32 {
            get: |e| e.beam_radius,
            set: |e, v| e.beam_radius = v,
            ui {
                min: 0.0,
                max: 10.0,
                format: "%.2f",
                tooltip: "Radius of the beam the arcs wrap around; 0 keeps a bare channel",
                group: "look",
            },
        },
        beam_arc_count: u32 {
            get: |e| e.beam_arc_count,
            set: |e, v| e.beam_arc_count = v,
            ui {
                min: 0.0,
                max: 64.0,
                format: "%.0f",
                group: "look",
            },
        },
        flash_gain: f32 {
            get: |e| e.flash_gain,
            set: |e, v| e.flash_gain = v,
            ui {
                min: 0.0,
                max: 10.0,
                format: "%.2f",
                tooltip: "Radiance of the ambient flash emitted while a stroke is alive; 0 disables it",
                group: "look",
            },
        },
        flash_radius: f32 {
            get: |e| e.flash_radius,
            set: |e, v| e.flash_radius = v,
            ui {
                min: 0.0,
                max: 100.0,
                format: "%.2f",
                group: "look",
            },
        },
        burst_start: f32 {
            get: |e| e.burst_start,
            set: |e, v| e.burst_start = v,
            ui {
                min: 0.0,
                max: 10.0,
                format: "%.2f",
                group: "timing",
            },
        },
        burst_interval: f32 {
            get: |e| e.burst_interval,
            set: |e, v| e.burst_interval = v,
            ui {
                min: 0.01,
                max: 10.0,
                format: "%.2f",
                group: "timing",
            },
        },
        burst_jitter: f32 {
            get: |e| e.burst_jitter,
            set: |e, v| e.burst_jitter = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Fraction of burst_interval the burst start is randomly shifted by",
                group: "timing",
            },
        },
        burst_count: u32 {
            get: |e| e.burst_count,
            set: |e, v| e.burst_count = v,
            ui {
                min: 0.0,
                max: 16.0,
                format: "%.0f",
                tooltip: "Number of bursts to play; 0 repeats forever",
                group: "timing",
            },
        },
        attack_time: f32 {
            get: |e| e.attack_time,
            set: |e, v| e.attack_time = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        sustain_time: f32 {
            get: |e| e.sustain_time,
            set: |e, v| e.sustain_time = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        release_time: f32 {
            get: |e| e.release_time,
            set: |e, v| e.release_time = v,
            ui {
                min: 0.0,
                max: 2.0,
                format: "%.3f",
                group: "timing",
            },
        },
        stroke_count: u32 {
            get: |e| e.stroke_count,
            set: |e, v| e.stroke_count = v,
            ui {
                min: 1.0,
                max: 8.0,
                format: "%.0f",
                tooltip: "Return strokes per burst, played stroke_interval apart",
                group: "timing",
            },
        },
        stroke_interval: f32 {
            get: |e| e.stroke_interval,
            set: |e, v| e.stroke_interval = v,
            ui {
                min: 0.001,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        stroke_decay: f32 {
            get: |e| e.stroke_decay,
            set: |e, v| e.stroke_decay = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                tooltip: "Intensity of each return stroke relative to the previous one",
                group: "timing",
            },
        },
        flicker_amplitude: f32 {
            get: |e| e.flicker_amplitude,
            set: |e, v| e.flicker_amplitude = v,
            ui {
                min: 0.0,
                max: 1.0,
                format: "%.2f",
                group: "timing",
            },
        },
        flicker_period: f32 {
            get: |e| e.flicker_period,
            set: |e, v| e.flicker_period = v,
            ui {
                min: 0.001,
                max: 1.0,
                format: "%.3f",
                group: "timing",
            },
        },
        reseed_level: u32 {
            get: |e| e.reseed_level,
            set: |e, v| e.reseed_level = v,
            ui {
                min: 0.0,
                max: 8.0,
                format: "%.0f",
                tooltip: "Subdivision level above which the channel is reshaped every reseed_period",
                group: "timing",
            },
        },
        reseed_period: f32 {
            get: |e| e.reseed_period,
            set: |e, v| e.reseed_period = v,
            ui {
                min: 0.01,
                max: 10.0,
                format: "%.2f",
                group: "timing",
            },
        },
        charge_ramp: f32 {
            get: |e| e.charge_ramp,
            set: |e, v| e.charge_ramp = v,
            ui {
                min: 0.0,
                max: 2.0,
                format: "%.2f",
                tooltip: "Seconds of dim leader glow before the first return stroke; 0 strikes instantly",
                group: "timing",
            },
        },
        seed: u32 {
            get: |e| e.seed,
            set: |e, v| e.seed = v,
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
        effect.source = LightningSource::Shell { radius: 2.5 };
        effect.core_radius = 0.12;

        let text = ron::ser::to_string_pretty(&effect, ron::ser::PrettyConfig::new())
            .expect("ron serialize");
        let restored: LightningEffect = ron::from_str(&text).expect("ron deserialize");
        assert_eq!(restored.source, LightningSource::Shell { radius: 2.5 });
        assert_eq!(restored.core_radius, 0.12);
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
        for name in ["core_color", "glow_color"] {
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
        loaded.core_radius = 0.2;
        loaded.time = 5.0;

        let mut target = LightningEffect::default();
        target.time = 2.5;

        overwrite_lightning_persisted_fields(&mut target, &loaded);
        assert_eq!(target.core_radius, 0.2);
        assert_eq!(target.time, 2.5);
    }
}
