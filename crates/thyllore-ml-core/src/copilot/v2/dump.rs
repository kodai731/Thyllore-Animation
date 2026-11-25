use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ndarray::Array1;
use ndarray_npy::write_npy;

pub struct V2CurveCopilotInferenceDump<'a> {
    pub context: &'a [f32],
    pub mean_curve: &'a [f32],
    pub fps: f32,
    pub anchor_time: f32,
}

pub fn dump_v2_curve_copilot_inference(
    dump: &V2CurveCopilotInferenceDump<'_>,
    output_dir: &Path,
) -> Result<PathBuf> {
    let dump_dir = output_dir.join("v2-curve-copilot-inference");
    create_dir_all(&dump_dir)?;

    write_npy(
        dump_dir.join("context.npy"),
        &Array1::from_vec(dump.context.to_vec()),
    )?;
    write_npy(
        dump_dir.join("mean_curve.npy"),
        &Array1::from_vec(dump.mean_curve.to_vec()),
    )?;
    write_npy(dump_dir.join("fps.npy"), &Array1::from_vec(vec![dump.fps]))?;
    write_npy(
        dump_dir.join("anchor_time.npy"),
        &Array1::from_vec(vec![dump.anchor_time]),
    )?;

    Ok(dump_dir)
}
