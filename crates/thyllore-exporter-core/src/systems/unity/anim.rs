use std::fmt::Write;

use crate::components::unity_anim::{UnityFloatCurve, UnityKey};

pub fn write_unity_anim(
    clip_name: &str,
    curves: &[UnityFloatCurve],
    stop_time: f32,
    loop_time: bool,
) -> String {
    let mut out = String::new();
    write_anim_header(&mut out);

    out.push_str("  m_ObjectHideFlags: 0\n");
    let _ = writeln!(out, "  m_Name: {}", clip_name);
    out.push_str("  serializedVersion: 7\n");
    out.push_str("  m_Legacy: 0\n");
    out.push_str("  m_Compressed: 0\n");
    out.push_str("  m_UseHighQualityCurve: 1\n");
    out.push_str("  m_RotationCurves: []\n");
    out.push_str("  m_CompressedRotationCurves: []\n");
    out.push_str("  m_EulerCurves: []\n");
    out.push_str("  m_PositionCurves: []\n");
    out.push_str("  m_ScaleCurves: []\n");
    write_float_curve_list(&mut out, "m_FloatCurves", curves);

    out.push_str("  m_PPtrCurves: []\n");
    out.push_str("  m_SampleRate: 60\n");
    out.push_str("  m_WrapMode: 0\n");
    out.push_str("  m_ClipBindingConstant:\n");
    out.push_str("    genericBindings: []\n");
    out.push_str("    pptrCurveMapping: []\n");
    write_clip_settings(&mut out, stop_time, loop_time);

    write_float_curve_list(&mut out, "m_EditorCurves", curves);
    out.push_str("  m_EulerEditorCurves: []\n");
    out.push_str("  m_HasGenericRootTransform: 0\n");
    out.push_str("  m_HasMotionFloatCurves: 0\n");
    out.push_str("  m_Events: []\n");
    out
}

fn write_anim_header(out: &mut String) {
    out.push_str("%YAML 1.1\n");
    out.push_str("%TAG !u! tag:unity3d.com,2011:\n");
    out.push_str("--- !u!74 &7400000\n");
    out.push_str("AnimationClip:\n");
}

fn write_float_curve_list(out: &mut String, key: &str, curves: &[UnityFloatCurve]) {
    if curves.is_empty() {
        let _ = writeln!(out, "  {}: []", key);
        return;
    }

    let _ = writeln!(out, "  {}:", key);
    for curve in curves {
        out.push_str("  - curve:\n");
        out.push_str("      serializedVersion: 2\n");
        out.push_str("      m_Curve:\n");
        write_curve_keys(out, &curve.keys);
        out.push_str("      m_PreInfinity: 2\n");
        out.push_str("      m_PostInfinity: 2\n");
        out.push_str("      m_RotationOrder: 4\n");
        let _ = writeln!(out, "    attribute: {}", curve.attribute);
        let _ = writeln!(out, "    path: {}", curve.path);
        out.push_str("    classID: 137\n");
        out.push_str("    script: {fileID: 0}\n");
    }
}

fn write_curve_keys(out: &mut String, keys: &[UnityKey]) {
    for key in keys {
        out.push_str("      - serializedVersion: 3\n");
        let _ = writeln!(out, "        time: {}", key.time);
        let _ = writeln!(out, "        value: {}", key.value);
        let _ = writeln!(out, "        inSlope: {}", format_slope(key.in_slope));
        let _ = writeln!(out, "        outSlope: {}", format_slope(key.out_slope));
        out.push_str("        tangentMode: 0\n");
        out.push_str("        weightedMode: 0\n");
        out.push_str("        inWeight: 0.33333334\n");
        out.push_str("        outWeight: 0.33333334\n");
    }
}

fn format_slope(slope: f32) -> String {
    if slope == f32::INFINITY {
        "Infinity".to_string()
    } else if slope == f32::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        slope.to_string()
    }
}

fn write_clip_settings(out: &mut String, stop_time: f32, loop_time: bool) {
    out.push_str("  m_AnimationClipSettings:\n");
    out.push_str("    serializedVersion: 2\n");
    out.push_str("    m_StartTime: 0\n");
    let _ = writeln!(out, "    m_StopTime: {}", stop_time);
    out.push_str("    m_OrientationOffsetY: 0\n");
    out.push_str("    m_Level: 0\n");
    out.push_str("    m_CycleOffset: 0\n");
    let _ = writeln!(out, "    m_LoopTime: {}", u8::from(loop_time));
    out.push_str("    m_LoopBlend: 0\n");
    out.push_str("    m_KeepOriginalPositionY: 1\n");
    out.push_str("    m_KeepOriginalPositionXZ: 0\n");
    out.push_str("    m_HeightFromFeet: 0\n");
    out.push_str("    m_Mirror: 0\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_unity_anim() {
        let curves = vec![UnityFloatCurve {
            path: "Body".to_string(),
            attribute: "blendShape.smile".to_string(),
            keys: vec![UnityKey {
                time: 0.0,
                value: 50.0,
                in_slope: 0.0,
                out_slope: 0.0,
            }],
        }];

        let yaml = write_unity_anim("TestClip", &curves, 1.0, false);

        let lines: Vec<&str> = yaml.lines().collect();
        let parseable = lines[3..].join("\n");

        let value: serde_yaml::Value = serde_yaml::from_str(&parseable).unwrap();

        let clip = value.get("AnimationClip").expect("missing AnimationClip");
        let float_curves = clip.get("m_FloatCurves").expect("missing m_FloatCurves");
        let first_curve = &float_curves[0];

        assert_eq!(
            first_curve.get("attribute").unwrap().as_str().unwrap(),
            "blendShape.smile"
        );
        assert_eq!(first_curve.get("path").unwrap().as_str().unwrap(), "Body");

        let curve_data = first_curve.get("curve").expect("missing curve");
        let m_curve = curve_data.get("m_Curve").expect("missing m_Curve");
        let first_key = &m_curve[0];
        assert_eq!(first_key.get("value").unwrap().as_f64().unwrap(), 50.0);

        let clip_settings = clip
            .get("m_AnimationClipSettings")
            .expect("missing m_AnimationClipSettings");
        assert_eq!(
            clip_settings.get("m_LoopTime").unwrap().as_i64().unwrap(),
            0
        );
    }
}
