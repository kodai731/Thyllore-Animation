use anyhow::{bail, Result};

use crate::ecs::component::WindTornadoEffect;

use super::flame_args::scalar_set_resolve_from_args;
use super::{
    BATCH_WIND_DEBUG_VIEW_FLAG, BATCH_WIND_MODE_FLAG, BATCH_WIND_RESOLVE_SCALE_FLAG,
    BATCH_WIND_SET_FLAG, BATCH_WIND_TIME_FLAG,
};

pub fn wind_mode_resolve_from_args(
    args: &[String],
) -> Result<Option<thyllore_effect_core::WindShadingMode>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_WIND_MODE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_WIND_MODE_FLAG} requires a value: closed|reference");
    };
    let mode = thyllore_effect_core::WindShadingMode::parse(value)
        .ok_or_else(|| anyhow::anyhow!("invalid wind mode '{value}': expected closed|reference"))?;
    Ok(Some(mode))
}

pub fn wind_debug_view_resolve_from_args(
    args: &[String],
) -> Result<Option<thyllore_effect_core::WindDebugView>> {
    let Some(position) = args
        .iter()
        .position(|arg| arg == BATCH_WIND_DEBUG_VIEW_FLAG)
    else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_WIND_DEBUG_VIEW_FLAG} requires a value: off|depth|knots|coverage");
    };
    let view = thyllore_effect_core::WindDebugView::parse(value).ok_or_else(|| {
        anyhow::anyhow!("invalid wind debug view '{value}': expected off|depth|knots|coverage")
    })?;
    Ok(Some(view))
}

pub fn wind_resolve_scale_resolve_from_args(
    args: &[String],
) -> Result<Option<thyllore_effect_core::WindResolveScale>> {
    let Some(position) = args
        .iter()
        .position(|arg| arg == BATCH_WIND_RESOLVE_SCALE_FLAG)
    else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_WIND_RESOLVE_SCALE_FLAG} requires a value: full|half");
    };
    let scale = thyllore_effect_core::WindResolveScale::parse(value).ok_or_else(|| {
        anyhow::anyhow!("invalid wind resolve scale '{value}': expected full|half")
    })?;
    Ok(Some(scale))
}

pub fn wind_fixed_time_resolve_from_args(args: &[String]) -> Result<Option<f32>> {
    let Some(position) = args.iter().position(|arg| arg == BATCH_WIND_TIME_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{BATCH_WIND_TIME_FLAG} requires a value (seconds)");
    };
    let seconds: f32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid wind time '{value}': expected float seconds"))?;
    Ok(Some(seconds))
}

pub(super) fn wind_set_valid_keys() -> Vec<&'static str> {
    thyllore_effect_core::WIND_SCALAR_PARAMS
        .iter()
        .map(|param| param.name)
        .collect()
}

pub(super) fn wind_set_resolve_from_args(args: &[String]) -> Result<Vec<(String, f32)>> {
    scalar_set_resolve_from_args(args, BATCH_WIND_SET_FLAG, &wind_set_valid_keys())
}

pub fn apply_wind_overrides(effect: &mut WindTornadoEffect, overrides: &[(String, f32)]) {
    for (key, value) in overrides {
        let param =
            thyllore_effect_core::find_scalar_param(thyllore_effect_core::WIND_SCALAR_PARAMS, key)
                .unwrap_or_else(|| unreachable!("unknown key (parser should have rejected)"));
        (param.set)(effect, *value);
    }
}
