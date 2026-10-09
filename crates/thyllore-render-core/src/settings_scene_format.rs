use thyllore_scene_core::declare_scene_format;

use crate::settings::{
    AutoExposure, BloomSettings, DepthOfField, Exposure, LensEffects, PhysicalCameraParameters,
    ToneMapOperator, ToneMapping,
};

declare_scene_format! {
    component: PhysicalCameraParameters,
    record: PhysicalCameraSceneRecord,
    items {
        key: "physical_camera",
        snapshot: physical_camera_parameter_snapshot,
        scalars: PHYSICAL_CAMERA_SCALAR_PARAMS,
        ui: PHYSICAL_CAMERA_UI_PARAMS,
        overwrite: overwrite_physical_camera_persisted_fields,
    },
    persisted {
        focal_length_mm: f32 { get: |p| p.focal_length_mm, set: |p, v| p.focal_length_mm = v },
        sensor_height_mm: f32 { get: |p| p.sensor_height_mm, set: |p, v| p.sensor_height_mm = v },
        aperture_f_stops: f32 { get: |p| p.aperture_f_stops, set: |p, v| p.aperture_f_stops = v },
        shutter_speed_s: f32 { get: |p| p.shutter_speed_s, set: |p, v| p.shutter_speed_s = v },
        sensitivity_iso: f32 { get: |p| p.sensitivity_iso, set: |p, v| p.sensitivity_iso = v },
    },
    runtime {},
}

declare_scene_format! {
    component: Exposure,
    record: ExposureSceneRecord,
    items {
        key: "exposure",
        snapshot: exposure_parameter_snapshot,
        scalars: EXPOSURE_SCALAR_PARAMS,
        ui: EXPOSURE_UI_PARAMS,
        overwrite: overwrite_exposure_persisted_fields,
    },
    persisted {
        ev100: f32 { get: |e| e.ev100, set: |e, v| e.ev100 = v },
        exposure_value: f32 { get: |e| e.exposure_value, set: |e, v| e.exposure_value = v },
    },
    runtime {},
}

declare_scene_format! {
    component: DepthOfField,
    record: DepthOfFieldSceneRecord,
    items {
        key: "depth_of_field",
        snapshot: depth_of_field_parameter_snapshot,
        scalars: DEPTH_OF_FIELD_SCALAR_PARAMS,
        ui: DEPTH_OF_FIELD_UI_PARAMS,
        overwrite: overwrite_depth_of_field_persisted_fields,
    },
    persisted {
        enabled: bool { get: |d| d.enabled, set: |d, v| d.enabled = v },
        focus_distance: f32 { get: |d| d.focus_distance, set: |d, v| d.focus_distance = v },
        max_blur_radius: f32 { get: |d| d.max_blur_radius, set: |d, v| d.max_blur_radius = v },
    },
    runtime {},
}

declare_scene_format! {
    component: ToneMapping,
    record: ToneMappingSceneRecord,
    items {
        key: "tone_mapping",
        snapshot: tone_mapping_parameter_snapshot,
        scalars: TONE_MAPPING_SCALAR_PARAMS,
        ui: TONE_MAPPING_UI_PARAMS,
        overwrite: overwrite_tone_mapping_persisted_fields,
    },
    persisted {
        enabled: bool { get: |t| t.enabled, set: |t, v| t.enabled = v },
        operator: String {
            get: |t| t.operator.name().to_string(),
            set: |t, v| {
                if let Some(operator) = ToneMapOperator::from_name(&v) {
                    t.operator = operator;
                }
            },
        },
        gamma: f32 { get: |t| t.gamma, set: |t, v| t.gamma = v },
    },
    runtime {},
}

declare_scene_format! {
    component: LensEffects,
    record: LensEffectsSceneRecord,
    items {
        key: "lens_effects",
        snapshot: lens_effects_parameter_snapshot,
        scalars: LENS_EFFECTS_SCALAR_PARAMS,
        ui: LENS_EFFECTS_UI_PARAMS,
        overwrite: overwrite_lens_effects_persisted_fields,
    },
    persisted {
        vignette_enabled: bool { get: |l| l.vignette_enabled, set: |l, v| l.vignette_enabled = v },
        vignette_intensity: f32 {
            get: |l| l.vignette_intensity,
            set: |l, v| l.vignette_intensity = v,
        },
        chromatic_aberration_enabled: bool {
            get: |l| l.chromatic_aberration_enabled,
            set: |l, v| l.chromatic_aberration_enabled = v,
        },
        chromatic_aberration_intensity: f32 {
            get: |l| l.chromatic_aberration_intensity,
            set: |l, v| l.chromatic_aberration_intensity = v,
        },
    },
    runtime {},
}

declare_scene_format! {
    component: BloomSettings,
    record: BloomSceneRecord,
    items {
        key: "bloom",
        snapshot: bloom_parameter_snapshot,
        scalars: BLOOM_SCALAR_PARAMS,
        ui: BLOOM_UI_PARAMS,
        overwrite: overwrite_bloom_persisted_fields,
    },
    persisted {
        enabled: bool { get: |b| b.enabled, set: |b, v| b.enabled = v },
        intensity: f32 { get: |b| b.intensity, set: |b, v| b.intensity = v },
        threshold: f32 { get: |b| b.threshold, set: |b, v| b.threshold = v },
        knee: f32 { get: |b| b.knee, set: |b, v| b.knee = v },
        mip_count: u32 { get: |b| b.mip_count, set: |b, v| b.mip_count = v },
    },
    runtime {},
}

declare_scene_format! {
    component: AutoExposure,
    record: AutoExposureSceneRecord,
    items {
        key: "auto_exposure",
        snapshot: auto_exposure_parameter_snapshot,
        scalars: AUTO_EXPOSURE_SCALAR_PARAMS,
        ui: AUTO_EXPOSURE_UI_PARAMS,
        overwrite: overwrite_auto_exposure_persisted_fields,
    },
    persisted {
        enabled: bool { get: |a| a.enabled, set: |a, v| a.enabled = v },
        min_ev: f32 { get: |a| a.min_ev, set: |a, v| a.min_ev = v },
        max_ev: f32 { get: |a| a.max_ev, set: |a, v| a.max_ev = v },
        adaptation_speed_up: f32 {
            get: |a| a.adaptation_speed_up,
            set: |a, v| a.adaptation_speed_up = v,
        },
        adaptation_speed_down: f32 {
            get: |a| a.adaptation_speed_down,
            set: |a, v| a.adaptation_speed_down = v,
        },
        low_percent: f32 { get: |a| a.low_percent, set: |a, v| a.low_percent = v },
        high_percent: f32 { get: |a| a.high_percent, set: |a, v| a.high_percent = v },
    },
    runtime {},
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_scene_core::SceneComponent;

    #[test]
    fn test_tone_mapping_roundtrip_keeps_the_operator_name_through_ron_value() {
        let mut settings = ToneMapping::default();
        settings.operator = ToneMapOperator::Reinhard;
        let text = ron::to_string(&settings).expect("serialize");
        assert!(text.contains("\"Reinhard\""), "{text}");
        let value: ron::Value = ron::from_str(&text).expect("value parses");
        let restored: ToneMapping = value.into_rust().expect("deserialize");
        assert_eq!(restored.operator, ToneMapOperator::Reinhard);
    }

    #[test]
    fn test_auto_exposure_keeps_runtime_fields_on_overwrite() {
        let mut target = AutoExposure::default();
        target.saved_manual_exposure = Some(2.0);
        target.min_log_luminance = -5.0;
        let mut loaded = AutoExposure::default();
        loaded.min_ev = 1.5;

        target.overwrite_persisted_fields(&loaded);
        assert_eq!(target.min_ev, 1.5);
        assert_eq!(target.saved_manual_exposure, Some(2.0));
        assert_eq!(target.min_log_luminance, -5.0);
        assert_eq!(AutoExposure::PERSISTED_FIELDS.len(), 7);
    }
}
