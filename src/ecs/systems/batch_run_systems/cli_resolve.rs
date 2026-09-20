use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::ecs::resource::{BatchRun, CaptureOutput, CaptureSchedule};
use crate::ecs::systems::cli_args::flag_value_resolve_from_args;

use super::anim_edits::{anim_edits_resolve_from_args, BatchAnimEdit};
use super::batch_action::BatchAction;
use super::debug_actions::debug_actions_resolve_from_args;

const BATCH_SCREENSHOT_FLAG: &str = "--batch-screenshot";
const BATCH_SCREENSHOT_SEQUENCE_FLAG: &str = "--batch-screenshot-sequence";
const BATCH_FRAMES_FLAG: &str = "--batch-frames";
const BATCH_CAMERA_FLAG: &str = "--batch-camera";
const GPU_TIMINGS_FLAG: &str = "--gpu-timings";
const EXPOSURE_DUMP_FLAG: &str = "--exposure-dump";
const BATCH_PICK_FLAG: &str = "--batch-pick";
const BATCH_WINDOW_FLAG: &str = "--batch-window";
const BATCH_SCENE_FLAG: &str = "--batch-scene";
const BATCH_PLAY_FLAG: &str = "--batch-play";
const BATCH_ANIM_DUMP_FLAG: &str = "--batch-anim-dump";
pub(super) const BATCH_ANIM_EDIT_FLAG: &str = "--batch-anim-edit";
pub(super) const BATCH_DEBUG_ACTION_FLAG: &str = "--batch-debug-action";
pub const BATCH_LIST_DEBUG_ACTIONS_FLAG: &str = "--batch-list-debug-actions";
pub(super) const DEFAULT_SCREENSHOT_FRAME: u64 = 120;

/// The engine's own startup flags; subsystem flags arrive through `bootstrap_hook!` and their
/// actions through `batch_action!`.
pub struct EngineCliOverrides {
    pub batch_run: Option<BatchRun>,
    pub camera_pose: Option<BatchCameraPose>,
    pub gpu_timings_path: Option<String>,
    pub exposure_dump_path: Option<String>,
    pub pick_pixel: Option<(u32, u32)>,
    pub window_size: Option<(u32, u32)>,
    pub batch_play: bool,
    pub scene_path: Option<String>,
    pub anim_edits: Vec<BatchAnimEdit>,
    pub anim_dump_path: Option<String>,
    pub debug_actions: Vec<Box<dyn BatchAction>>,
}

pub fn resolve_engine_cli_overrides(args: &[String]) -> Result<EngineCliOverrides> {
    Ok(EngineCliOverrides {
        batch_run: batch_run_resolve_from_args(args)?,
        camera_pose: camera_pose_resolve_from_args(args)?,
        gpu_timings_path: gpu_timings_path_resolve_from_args(args)?,
        exposure_dump_path: exposure_dump_path_resolve_from_args(args)?,
        pick_pixel: pick_pixel_resolve_from_args(args)?,
        window_size: window_size_resolve_from_args(args)?,
        batch_play: args.iter().any(|a| a == BATCH_PLAY_FLAG),
        scene_path: scene_path_resolve_from_args(args)?,
        anim_edits: anim_edits_resolve_from_args(args)?,
        anim_dump_path: flag_value_resolve_from_args(args, BATCH_ANIM_DUMP_FLAG)?,
        debug_actions: debug_actions_resolve_from_args(args)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BatchCameraPose {
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub distance: f32,
    pub pivot: Option<[f32; 3]>,
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

fn screenshot_frame_resolve_from_args(args: &[String]) -> Result<u64> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_FRAMES_FLAG) else {
        return Ok(DEFAULT_SCREENSHOT_FRAME);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_FRAMES_FLAG} requires a frame count");
    };
    let frames: u64 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid frame count '{value}': expected integer"))?;
    if frames == 0 {
        bail!("{BATCH_FRAMES_FLAG} must be >= 1");
    }
    Ok(frames)
}

fn sequence_batch_resolve(value: &str, first_frame: u64) -> Result<BatchRun> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 3 {
        bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} expects <dir>,<count>,<stride>, got '{value}'");
    }
    let dir = parts[0].trim();
    let count: u32 = parts[1]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid count '{}': expected positive integer", parts[1]))?;
    let stride: u32 = parts[2]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid stride '{}': expected positive integer", parts[2]))?;
    if count == 0 {
        bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} count must be >= 1");
    }
    if stride == 0 {
        bail!("{BATCH_SCREENSHOT_SEQUENCE_FLAG} stride must be >= 1");
    }

    Ok(BatchRun::new(CaptureSchedule {
        first_frame,
        stride,
        count,
        output: CaptureOutput::Sequence(PathBuf::from(dir)),
    }))
}

pub fn batch_run_resolve_from_args(args: &[String]) -> Result<Option<BatchRun>> {
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
        let screenshot_frame = screenshot_frame_resolve_from_args(args)?;
        return Ok(Some(sequence_batch_resolve(value, screenshot_frame)?));
    }

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
    let screenshot_frame = screenshot_frame_resolve_from_args(args)?;
    Ok(Some(BatchRun::new(CaptureSchedule::single(
        output,
        screenshot_frame,
    ))))
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
    let Some(position) = args.iter().position(|arg| arg == BATCH_SCENE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_SCENE_FLAG} requires <path>");
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

/// Sizes the startup window from the command line so a headless run renders at the resolution the
/// reference frames were captured at.
pub fn window_size_resolve_from_args(args: &[String]) -> Result<Option<(u32, u32)>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_WINDOW_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_WINDOW_FLAG} requires <width>,<height>");
    };
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 2 {
        bail!("{BATCH_WINDOW_FLAG} expects 2 comma-separated values, got '{value}'");
    }
    let width: u32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {BATCH_WINDOW_FLAG} width in '{value}'"))?;
    let height: u32 = parts[1]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {BATCH_WINDOW_FLAG} height in '{value}'"))?;
    if width == 0 || height == 0 {
        bail!("{BATCH_WINDOW_FLAG} expects non-zero extents, got '{value}'");
    }
    Ok(Some((width, height)))
}
