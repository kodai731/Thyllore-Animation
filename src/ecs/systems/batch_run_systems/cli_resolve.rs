use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::ecs::resource::{BatchDumpPlan, BatchRun};

use super::debug_actions::{debug_actions_has_wall_probe_dump, debug_actions_has_water_debug_dump};
use super::flame_args::flame_set_resolve_from_args;
use super::{
    BATCH_CAMERA_FLAG, BATCH_FLAME_TRACE_FLAG, BATCH_FRAMES_FLAG, BATCH_PICK_FLAG,
    BATCH_SCREENSHOT_FLAG, BATCH_SCREENSHOT_SEQUENCE_FLAG, BATCH_WALL_PROBE_FLAG,
    BATCH_WATER_PROBE_FLAG, DEFAULT_SCREENSHOT_FRAME, EXPOSURE_DUMP_FLAG, GPU_TIMINGS_FLAG,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BatchCameraPose {
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub distance: f32,
    pub pivot: Option<[f32; 3]>,
}

fn resolve_absolute_output(output: &Path) -> Result<PathBuf> {
    if output.extension().and_then(|e| e.to_str()) != Some("png") {
        bail!(
            "batch screenshot output must end with .png: {}",
            output.display()
        );
    }
    if output.is_absolute() {
        return Ok(output.to_path_buf());
    }
    Ok(std::env::current_dir()?.join(output))
}

pub fn flag_value_resolve_from_args(args: &[String], flag: &str) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1).filter(|v| !v.starts_with("--")) else {
        bail!("{flag} requires a value");
    };
    Ok(Some(value.clone()))
}

pub fn camera_pose_resolve_from_args(args: &[String]) -> Result<Option<BatchCameraPose>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_CAMERA_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_CAMERA_FLAG} requires <yaw_deg>,<pitch_deg>,<distance>");
    };

    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 3 && parts.len() != 6 {
        bail!(
            "{BATCH_CAMERA_FLAG} expects <yaw>,<pitch>,<distance>[,<pivot_x>,<pivot_y>,<pivot_z>], got '{value}'"
        );
    }
    let numbers: Vec<f32> = parts
        .iter()
        .map(|part| part.trim().parse::<f32>())
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("invalid {BATCH_CAMERA_FLAG} value '{value}'"))?;
    if !numbers.iter().all(|n| n.is_finite()) || numbers[2] <= 0.0 {
        bail!("{BATCH_CAMERA_FLAG} distance must be > 0 and all values finite: '{value}'");
    }

    Ok(Some(BatchCameraPose {
        yaw_degrees: numbers[0],
        pitch_degrees: numbers[1],
        distance: numbers[2],
        pivot: (numbers.len() == 6).then(|| [numbers[3], numbers[4], numbers[5]]),
    }))
}

pub fn batch_run_resolve_from_args(args: &[String]) -> Result<Option<(BatchRun, BatchDumpPlan)>> {
    // Check for sequence mode first
    if let Some(sequence_position) = args
        .iter()
        .position(|arg| arg == BATCH_SCREENSHOT_SEQUENCE_FLAG)
    {
        let Some(value) = args
            .get(sequence_position + 1)
            .filter(|v| !v.starts_with("--"))
        else {
            bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} requires <dir>,<count>,<stride>");
        };
        let parts: Vec<&str> = value.split(',').collect();
        if parts.len() != 3 {
            bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} expects <dir>,<count>,<stride>, got '{value}'");
        }
        let dir = parts[0].trim();
        let count: u32 = parts[1].trim().parse().map_err(|_| {
            anyhow::anyhow!("invalid count '{}': expected positive integer", parts[1])
        })?;
        let stride: u32 = parts[2].trim().parse().map_err(|_| {
            anyhow::anyhow!("invalid stride '{}': expected positive integer", parts[2])
        })?;
        if count == 0 {
            bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} count must be >= 1");
        }
        if stride == 0 {
            bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} stride must be >= 1");
        }

        let screenshot_frame = match args.iter().position(|arg| arg == BATCH_FRAMES_FLAG) {
            Some(frames_position) => {
                let Some(value) = args.get(frames_position + 1) else {
                    bail!("{BATCH_FRAMES_FLAG} requires a frame count");
                };
                let frames: u64 = value.parse().map_err(|_| {
                    anyhow::anyhow!("invalid frame count '{value}': expected integer")
                })?;
                if frames == 0 {
                    bail!("{BATCH_FRAMES_FLAG} must be >= 1");
                }
                frames
            }
            None => DEFAULT_SCREENSHOT_FRAME,
        };

        let flame_set = flame_set_resolve_from_args(args)?;
        let dump_wall_probe = debug_actions_has_wall_probe_dump(args);
        let dump_water_debug = debug_actions_has_water_debug_dump(args);

        let mut batch = BatchRun::new(PathBuf::from(dir), screenshot_frame);
        batch.captures_remaining = count;
        batch.stride = stride;
        batch.sequence_dir = Some(PathBuf::from(dir));
        batch.total_count = count;

        let dump_plan = BatchDumpPlan {
            flame_set,
            dump_wall_probe,
            dump_water_debug,
            flame_trace_path: flag_value_resolve_from_args(args, BATCH_FLAME_TRACE_FLAG)?
                .map(PathBuf::from),
            wall_probe_path: flag_value_resolve_from_args(args, BATCH_WALL_PROBE_FLAG)?
                .map(PathBuf::from),
            water_probe_path: flag_value_resolve_from_args(args, BATCH_WATER_PROBE_FLAG)?
                .map(PathBuf::from),
        };

        Ok(Some((batch, dump_plan)))
    } else {
        // Single-shot mode (existing behavior)
        let Some(position) = args.iter().position(|arg| arg == BATCH_SCREENSHOT_FLAG) else {
            if args.iter().any(|arg| arg == BATCH_FRAMES_FLAG) {
                bail!("{BATCH_FRAMES_FLAG} requires {BATCH_SCREENSHOT_FLAG} <output.png>");
            }
            return Ok(None);
        };

        let Some(output) = args.get(position + 1).filter(|v| !v.starts_with("--")) else {
            bail!("{BATCH_SCREENSHOT_FLAG} requires an output path");
        };
        let output = resolve_absolute_output(Path::new(output))?;

        let screenshot_frame = match args.iter().position(|arg| arg == BATCH_FRAMES_FLAG) {
            Some(frames_position) => {
                let Some(value) = args.get(frames_position + 1) else {
                    bail!("{BATCH_FRAMES_FLAG} requires a frame count");
                };
                let frames: u64 = value.parse().map_err(|_| {
                    anyhow::anyhow!("invalid frame count '{value}': expected integer")
                })?;
                if frames == 0 {
                    bail!("{BATCH_FRAMES_FLAG} must be >= 1");
                }
                frames
            }
            None => DEFAULT_SCREENSHOT_FRAME,
        };

        let flame_set = flame_set_resolve_from_args(args)?;

        let dump_wall_probe = debug_actions_has_wall_probe_dump(args);
        let dump_water_debug = debug_actions_has_water_debug_dump(args);

        let batch = BatchRun::new(output, screenshot_frame);

        let dump_plan = BatchDumpPlan {
            flame_set,
            dump_wall_probe,
            dump_water_debug,
            flame_trace_path: flag_value_resolve_from_args(args, BATCH_FLAME_TRACE_FLAG)?
                .map(PathBuf::from),
            wall_probe_path: flag_value_resolve_from_args(args, BATCH_WALL_PROBE_FLAG)?
                .map(PathBuf::from),
            water_probe_path: flag_value_resolve_from_args(args, BATCH_WATER_PROBE_FLAG)?
                .map(PathBuf::from),
        };

        Ok(Some((batch, dump_plan)))
    }
}

pub fn gpu_timings_path_resolve_from_args(args: &[String]) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == GPU_TIMINGS_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{GPU_TIMINGS_FLAG} requires a path");
    };
    Ok(Some(value.clone()))
}

pub fn exposure_dump_path_resolve_from_args(args: &[String]) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == EXPOSURE_DUMP_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{EXPOSURE_DUMP_FLAG} requires a path");
    };
    Ok(Some(value.clone()))
}

pub fn scene_path_resolve_from_args(args: &[String]) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == "--batch-scene") else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("--batch-scene requires <path>");
    };
    Ok(Some(value.clone()))
}

/// Drives one viewport click from the command line so the picking readback path can be exercised
/// headlessly, where there is no mouse to press.
pub fn pick_pixel_resolve_from_args(args: &[String]) -> Result<Option<(u32, u32)>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_PICK_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_PICK_FLAG} requires <x>,<y>");
    };
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 2 {
        bail!("{BATCH_PICK_FLAG} expects 2 comma-separated values, got '{value}'");
    }
    let x: u32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {BATCH_PICK_FLAG} x in '{value}'"))?;
    let y: u32 = parts[1]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {BATCH_PICK_FLAG} y in '{value}'"))?;
    Ok(Some((x, y)))
}
