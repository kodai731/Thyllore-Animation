use std::fs;
use std::path::Path;

use crate::muscle::components::muscle_curves::MuscleCurves;
use crate::muscle::components::unity_muscle_table::UnityMuscleTable;

const TEMPLATE: &str = include_str!("../../../data/unity_anim_clip.yaml");

fn compute_tangents(times: &[f32], values: &[f32]) -> Vec<(f32, f32)> {
    let n = times.len();
    let mut tangents = Vec::with_capacity(n);
    for i in 0..n {
        let in_slope = if i > 0 {
            (values[i] - values[i - 1]) / (times[i] - times[i - 1])
        } else {
            0.0
        };
        let out_slope = if i < n - 1 {
            (values[i + 1] - values[i]) / (times[i + 1] - times[i])
        } else {
            0.0
        };
        tangents.push((in_slope, out_slope));
    }
    tangents
}

fn build_curve_entries(curves: &MuscleCurves, table: &UnityMuscleTable) -> Vec<String> {
    let mut entries = Vec::new();

    for muscle in &table.muscles {
        let i = muscle.index;

        let values: Vec<f32> = curves.frames.iter().map(|frame| frame[i]).collect();
        let tangents = compute_tangents(&curves.times, &values);

        let mut curve_keys = Vec::new();
        for (k, (&time, &value)) in curves.times.iter().zip(values.iter()).enumerate() {
            let (in_slope, out_slope) = tangents[k];

            curve_keys.push(format!(
                "      - serializedVersion: 3\n        time: {}\n        value: {}\n        inSlope: {}\n        outSlope: {}\n        tangentMode: 0\n        weightedMode: 0\n        inWeight: 0.33333334\n        outWeight: 0.33333334",
                time,
                value,
                in_slope,
                out_slope,
            ));
        }

        let curve_keys_str = curve_keys.join("\n");
        entries.push(format!(
            "  - curve:\n      serializedVersion: 2\n      m_Curve:\n{}\n      m_PreInfinity: 2\n      m_PostInfinity: 2\n      m_RotationOrder: 4\n    attribute: {}\n    path: \n    classID: 95\n    script: {{fileID: 0}}",
            curve_keys_str,
            muscle.attribute,
        ));
    }

    entries
}

pub fn unity_anim_text(
    name: &str,
    curves: &MuscleCurves,
    table: &UnityMuscleTable,
    loop_time: bool,
) -> String {
    let curve_entries = build_curve_entries(curves, table);
    let curves_str = curve_entries.join("\n");

    let stop_time = format!("{}", curves.duration);
    let loop_time_val = if loop_time { 1 } else { 0 };

    TEMPLATE
        .replace("{name}", name)
        .replace("{sample_rate}", &curves.sample_rate.to_string())
        .replace("{stop_time}", &stop_time)
        .replace("{loop_time}", &loop_time_val.to_string())
        .replace("{curves}", &format!("{}\n", curves_str))
}

pub fn write_unity_anim(
    path: &Path,
    name: &str,
    curves: &MuscleCurves,
    table: &UnityMuscleTable,
    loop_time: bool,
) -> std::io::Result<()> {
    let text = unity_anim_text(name, curves, table, loop_time);
    fs::write(path, text)?;
    Ok(())
}
