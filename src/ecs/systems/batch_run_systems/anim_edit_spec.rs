use std::path::PathBuf;

use anyhow::{bail, Result};

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{scalar_channel_for_cli_name, scalar_cli_names_joined};
use crate::ecs::resource::{BatchAnimEdit, BoneAxis};

use super::flags::BATCH_ANIM_EDIT_FLAG;
use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSpec;

/// Parse repeated `--batch-anim-edit <spec>` flags. Specs:
/// `debug_keys=<seed>` | `key=<param>@<time>=<value>` | `key=<bone_name>.<x|y|z|tx|ty|tz>@<time>=<value>` | `clear`.
pub(super) fn anim_edits_resolve_from_args(args: &[String]) -> Result<Vec<BatchAnimEdit>> {
    let mut edits = Vec::new();
    for i in 0..args.len() {
        if args[i] != BATCH_ANIM_EDIT_FLAG {
            continue;
        }
        let Some(spec) = args.get(i + 1).filter(|v| !v.starts_with("--")) else {
            bail!("{BATCH_ANIM_EDIT_FLAG} requires a spec: debug_keys=<seed> | key=<param>@<time>=<value> | key=<bone_name>.<x|y|z|tx|ty|tz>@<time>=<value> | key_at_playhead=<param> | trim_end=<seconds> | new_clip=<name> | template=<path> | save=<path> | compose=<motion>[,side=left|right][,count=<n>][,amount=<f>][,speed=<f>] | copilot_extend=<bone_name>.<x|y|z>@<time>,<frames> | clear");
        };
        edits.push(anim_edit_parse_spec(spec)?);
    }
    Ok(edits)
}

pub(super) fn anim_edit_parse_spec(spec: &str) -> Result<BatchAnimEdit> {
    let spec = spec.trim();
    if spec == "clear" {
        return Ok(BatchAnimEdit::Clear);
    }
    if let Some(seed_str) = spec.strip_prefix("debug_keys=") {
        let seed: u64 = seed_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid debug_keys seed '{seed_str}': expected u64"))?;
        return Ok(BatchAnimEdit::DebugKeys { seed });
    }
    if let Some(param_str) = spec.strip_prefix("key_at_playhead=") {
        return Ok(BatchAnimEdit::KeyAtPlayhead {
            property_type: scalar_property_for_cli_name(param_str)?,
        });
    }
    if let Some(seconds_str) = spec.strip_prefix("trim_end=") {
        let seconds: f32 = seconds_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid trim_end seconds '{seconds_str}'"))?;
        if !seconds.is_finite() || seconds < 0.0 {
            bail!("trim_end seconds must be >= 0 and finite: '{spec}'");
        }
        return Ok(BatchAnimEdit::TrimEnd { seconds });
    }
    if let Some(rest) = spec.strip_prefix("key=") {
        let (param_str, rest) = rest.split_once('@').ok_or_else(|| {
            anyhow::anyhow!("key spec must be key=<bone_name>.<axis>@<time>=<value>, got '{spec}'")
        })?;
        let (time_str, value_str) = rest.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("key spec must be key=<bone_name>.<axis>@<time>=<value>, got '{spec}'")
        })?;
        let time: f32 = time_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid key time '{time_str}'"))?;
        let value: f32 = value_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid key value '{value_str}'"))?;
        if !time.is_finite() || time < 0.0 || !value.is_finite() {
            bail!("key time must be >= 0 and value finite: '{spec}'");
        }
        if let Some(dot) = param_str.find('.') {
            let bone_name = &param_str[..dot];
            let axis_str = &param_str[dot + 1..];
            let axis = parse_bone_axis(axis_str, bone_name)?;
            return Ok(BatchAnimEdit::BoneKey {
                bone_name: bone_name.to_string(),
                axis,
                time,
                value,
            });
        }
        let property_type = scalar_property_for_cli_name(param_str)?;
        return Ok(BatchAnimEdit::Key {
            property_type,
            time,
            value,
        });
    }
    if let Some(name) = spec.strip_prefix("new_clip=") {
        let name = name.trim();
        if name.is_empty() {
            bail!("new_clip name must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::NewClip {
            name: name.to_string(),
        });
    }
    if let Some(path_str) = spec.strip_prefix("template=") {
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            bail!("template path must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::Template { path });
    }
    if let Some(path_str) = spec.strip_prefix("save=") {
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            bail!("save path must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::Save { path });
    }
    if let Some(rest) = spec.strip_prefix("compose=") {
        return Ok(BatchAnimEdit::Compose {
            spec: MotionSpec::parse(rest)?,
        });
    }
    if let Some(rest) = spec.strip_prefix("copilot_extend=") {
        let (bone_axis, frames_str) = rest.split_once(',').ok_or_else(|| {
            anyhow::anyhow!("copilot_extend spec must be copilot_extend=<bone_name>.<x|y|z>@<time>,<frames>, got '{spec}'")
        })?;
        let (param_str, time_str) = bone_axis.split_once('@').ok_or_else(|| {
            anyhow::anyhow!("copilot_extend spec must be copilot_extend=<bone_name>.<x|y|z>@<time>,<frames>, got '{spec}'")
        })?;
        let time: f32 = time_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid copilot_extend time '{}'", time_str))?;
        let frames: usize = frames_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid copilot_extend frames '{}'", frames_str))?;
        if !time.is_finite() || time < 0.0 {
            bail!("copilot_extend time must be >= 0 and finite: '{spec}'");
        }
        let dot = param_str.find('.').ok_or_else(|| {
            anyhow::anyhow!(
                "copilot_extend bone_name must have a dot (e.g. Hips.x), got '{param_str}'"
            )
        })?;
        let bone_name = &param_str[..dot];
        let axis_str = &param_str[dot + 1..];
        let axis = parse_bone_axis(axis_str, bone_name)?;
        match axis {
            BoneAxis::RotationX | BoneAxis::RotationY | BoneAxis::RotationZ => {}
            BoneAxis::TranslationX | BoneAxis::TranslationY | BoneAxis::TranslationZ => {
                bail!("copilot_extend axis must be x, y or z: '{spec}'");
            }
        }
        return Ok(BatchAnimEdit::CopilotExtend {
            bone_name: bone_name.to_string(),
            axis,
            time,
            frames,
        });
    }
    bail!("unknown anim edit spec '{spec}'. Expected debug_keys=<seed> | key=<param>@<time>=<value> | key_at_playhead=<param> | trim_end=<seconds> | new_clip=<name> | template=<path> | save=<path> | compose=<motion>[,side=left|right][,count=<n>][,amount=<f>][,speed=<f>] | copilot_extend=<bone_name>.<x|y|z>@<time>,<frames> | clear")
}

fn scalar_property_for_cli_name(name: &str) -> Result<PropertyType> {
    let (domain, channel) = scalar_channel_for_cli_name(name.trim()).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown scalar channel '{}'. Valid channels: {}",
            name,
            scalar_cli_names_joined()
        )
    })?;
    domain
        .property_type_of(channel)
        .ok_or_else(|| anyhow::anyhow!("scalar channel '{name}' is not in its domain table"))
}

fn parse_bone_axis(axis_str: &str, bone_name: &str) -> Result<BoneAxis> {
    use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
    let allows_translation =
        HumanoidRole::from_unity_name(bone_name).map_or(true, |role| role.allows_translation());
    match axis_str {
        "x" => Ok(BoneAxis::RotationX),
        "y" => Ok(BoneAxis::RotationY),
        "z" => Ok(BoneAxis::RotationZ),
        "tx" if allows_translation => Ok(BoneAxis::TranslationX),
        "ty" if allows_translation => Ok(BoneAxis::TranslationY),
        "tz" if allows_translation => Ok(BoneAxis::TranslationZ),
        other => Err(anyhow::anyhow!(
            "invalid axis '{}' for bone '{}'. Valid axes: x, y, z (tx, ty, tz not for roles without translation)",
            other,
            bone_name
        )),
    }
}
