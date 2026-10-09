use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::ecs::component::{scalar_channel_for_cli_name, scalar_cli_names_joined};
use crate::ecs::resource::{BatchAnimEdit, BoneAxis};
use thyllore_anim_core::editable::PropertyType;
use thyllore_cli_core::{finite_float_parse, nonnegative_finite_float_parse, required_split};

use super::flags::BATCH_ANIM_EDIT_FLAG;
use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSpec;

const SPEC_USAGE: &str = "debug_keys=<seed> | key=<param>@<time>=<value> | key=<bone_name>.<x|y|z|tx|ty|tz>@<time>=<value> | key_at_playhead=<param> | trim_end=<seconds> | new_clip=<name> | template=<path> | save=<path> | compose=<motion>[,side=left|right][,count=<n>][,amount=<f>][,speed=<f>] | copilot_extend=<bone_name>.<x|y|z>@<time>,<frames> | clear";

pub(super) fn anim_edits_resolve_from_args(args: &[String]) -> Result<Vec<BatchAnimEdit>> {
    let mut edits = Vec::new();
    for i in 0..args.len() {
        if args[i] != BATCH_ANIM_EDIT_FLAG {
            continue;
        }
        let Some(spec) = args.get(i + 1).filter(|v| !v.starts_with("--")) else {
            bail!("{BATCH_ANIM_EDIT_FLAG} requires a spec: {SPEC_USAGE}");
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
    let (kind, rest) = spec
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("unknown anim edit spec '{spec}'. Expected {SPEC_USAGE}"))?;
    match kind {
        "debug_keys" => parse_debug_keys(rest),
        "key" => parse_key(rest),
        "key_at_playhead" => Ok(BatchAnimEdit::KeyAtPlayhead {
            property_type: scalar_property_for_cli_name(rest)?,
        }),
        "trim_end" => parse_trim_end(rest),
        "new_clip" => parse_new_clip(rest),
        "template" => parse_template(rest),
        "save" => parse_save(rest),
        "compose" => Ok(BatchAnimEdit::Compose {
            spec: MotionSpec::parse(rest)?,
        }),
        "copilot_extend" => parse_copilot_extend(rest),
        unknown => bail!("unknown anim edit spec '{unknown}=<value>'. Expected {SPEC_USAGE}"),
    }
}

fn parse_debug_keys(rest: &str) -> Result<BatchAnimEdit> {
    let seed: u64 = rest
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid debug_keys seed '{}': expected u64", rest.trim()))?;
    Ok(BatchAnimEdit::DebugKeys { seed })
}

fn parse_key(rest: &str) -> Result<BatchAnimEdit> {
    let (param_str, time_value) =
        required_split(rest, '@', "key=<bone_name>.<axis>@<time>=<value>")
            .map_err(anyhow::Error::msg)?;
    let (time_str, value_str) =
        required_split(time_value, '=', "key=<bone_name>.<axis>@<time>=<value>")
            .map_err(anyhow::Error::msg)?;
    let time = nonnegative_finite_float_parse(time_str.trim()).map_err(anyhow::Error::msg)?;
    let value = finite_float_parse(value_str.trim()).map_err(anyhow::Error::msg)?;
    if param_str.contains('.') {
        let (bone_name, axis) = parse_bone_and_axis(param_str)?;
        Ok(BatchAnimEdit::BoneKey {
            bone_name: bone_name.to_string(),
            axis,
            time,
            value,
        })
    } else {
        let property_type = scalar_property_for_cli_name(param_str)?;
        Ok(BatchAnimEdit::Key {
            property_type,
            time,
            value,
        })
    }
}

fn parse_trim_end(rest: &str) -> Result<BatchAnimEdit> {
    let seconds = nonnegative_finite_float_parse(rest.trim()).map_err(anyhow::Error::msg)?;
    Ok(BatchAnimEdit::TrimEnd { seconds })
}

fn parse_new_clip(rest: &str) -> Result<BatchAnimEdit> {
    let name = rest.trim();
    if name.is_empty() {
        bail!("new_clip name must not be empty");
    }
    Ok(BatchAnimEdit::NewClip {
        name: name.to_string(),
    })
}

fn parse_template(rest: &str) -> Result<BatchAnimEdit> {
    let path = PathBuf::from(rest.trim());
    if path.as_os_str().is_empty() {
        bail!("template path must not be empty");
    }
    Ok(BatchAnimEdit::Template { path })
}

fn parse_save(rest: &str) -> Result<BatchAnimEdit> {
    let path = PathBuf::from(rest.trim());
    if path.as_os_str().is_empty() {
        bail!("save path must not be empty");
    }
    Ok(BatchAnimEdit::Save { path })
}

fn parse_copilot_extend(rest: &str) -> Result<BatchAnimEdit> {
    let (bone_time, frames_str) = required_split(
        rest,
        ',',
        "copilot_extend=<bone_name>.<x|y|z>@<time>,<frames>",
    )
    .map_err(anyhow::Error::msg)?;
    let (param_str, time_str) = required_split(
        bone_time,
        '@',
        "copilot_extend=<bone_name>.<x|y|z>@<time>,<frames>",
    )
    .map_err(anyhow::Error::msg)?;
    let time = nonnegative_finite_float_parse(time_str.trim()).map_err(anyhow::Error::msg)?;
    let frames: usize = frames_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid copilot_extend frames '{}'", frames_str.trim()))?;
    let (bone_name, axis) = parse_bone_and_axis(param_str)?;
    match axis {
        BoneAxis::RotationX | BoneAxis::RotationY | BoneAxis::RotationZ => {}
        BoneAxis::TranslationX | BoneAxis::TranslationY | BoneAxis::TranslationZ => {
            bail!("copilot_extend axis must be x, y or z");
        }
    }
    Ok(BatchAnimEdit::CopilotExtend {
        bone_name: bone_name.to_string(),
        axis,
        time,
        frames,
    })
}

fn parse_bone_and_axis(param_str: &str) -> Result<(&str, BoneAxis)> {
    let dot = param_str.find('.').ok_or_else(|| {
        anyhow::anyhow!(
            "bone_name must have a dot (e.g. Hips.x), got '{}'",
            param_str
        )
    })?;
    let bone_name = &param_str[..dot];
    let axis_str = &param_str[dot + 1..];
    let axis = parse_bone_axis(axis_str, bone_name)?;
    Ok((bone_name, axis))
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
