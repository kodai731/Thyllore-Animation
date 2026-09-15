use anyhow::{bail, Result};

use crate::ecs::component::LightningEffect;

use super::flame_args::scalar_set_resolve_from_args;
use super::{
    BATCH_LIGHTNING_DEBUG_VIEW_FLAG, BATCH_LIGHTNING_MODE_FLAG, BATCH_LIGHTNING_SET_FLAG,
    BATCH_LIGHTNING_TIME_FLAG,
};

pub fn lightning_mode_resolve_from_args(
    args: &[String],
) -> Result<Option<thyllore_effect_core::LightningShadingMode>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_LIGHTNING_MODE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_LIGHTNING_MODE_FLAG} requires a value: closed|reference");
    };
    let mode = thyllore_effect_core::LightningShadingMode::parse(value).ok_or_else(|| {
        anyhow::anyhow!("invalid lightning mode '{value}': expected closed|reference")
    })?;
    Ok(Some(mode))
}

pub fn lightning_debug_view_resolve_from_args(
    args: &[String],
) -> Result<Option<thyllore_effect_core::LightningDebugView>> {
    let Some(position) = args
        .iter()
        .position(|arg| arg == BATCH_LIGHTNING_DEBUG_VIEW_FLAG)
    else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_LIGHTNING_DEBUG_VIEW_FLAG} requires a value: off|coverage|hits|core");
    };
    let view = thyllore_effect_core::LightningDebugView::parse(value).ok_or_else(|| {
        anyhow::anyhow!("invalid lightning debug view '{value}': expected off|coverage|hits|core")
    })?;
    Ok(Some(view))
}

pub fn lightning_fixed_time_resolve_from_args(args: &[String]) -> Result<Option<f32>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_LIGHTNING_TIME_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_LIGHTNING_TIME_FLAG} requires a value (seconds)");
    };
    let seconds: f32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid lightning time '{value}': expected float seconds"))?;
    Ok(Some(seconds))
}

pub(super) fn lightning_set_valid_keys() -> Vec<&'static str> {
    thyllore_effect_core::LIGHTNING_SCALAR_PARAMS
        .iter()
        .map(|param| param.name)
        .collect()
}

pub(super) fn lightning_set_resolve_from_args(args: &[String]) -> Result<Vec<(String, f32)>> {
    scalar_set_resolve_from_args(args, BATCH_LIGHTNING_SET_FLAG, &lightning_set_valid_keys())
}

pub fn apply_lightning_overrides(effect: &mut LightningEffect, overrides: &[(String, f32)]) {
    for (key, value) in overrides {
        let param = thyllore_effect_core::find_scalar_param(
            thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
            key,
        )
        .unwrap_or_else(|| unreachable!("unknown key (parser should have rejected)"));
        (param.set)(effect, *value);
    }
}
