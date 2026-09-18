use anyhow::{bail, Result};

pub fn flag_value_resolve_from_args(args: &[String], flag: &str) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1).filter(|v| !v.starts_with("--")) else {
        bail!("{flag} requires a value");
    };
    Ok(Some(value.clone()))
}

/// Parses one `KEY=VALUE` flag value, rejecting keys outside `valid_keys`.
pub fn scalar_assignment_parse(
    text: &str,
    valid_keys: &[&'static str],
) -> Result<(String, f32), String> {
    let Some((key, value_text)) = text.split_once('=') else {
        return Err(format!("expected KEY=VALUE, got '{text}'"));
    };
    let key = key.trim();
    let value_text = value_text.trim();

    let value: f32 = value_text
        .parse()
        .map_err(|_| format!("value must be a number, got '{value_text}'"))?;
    if !valid_keys.contains(&key) {
        return Err(format!(
            "unknown key '{key}'. Valid keys: {}",
            valid_keys.join(", ")
        ));
    }
    Ok((key.to_string(), value))
}

/// Parses `<a>,<b>` into two finite floats.
pub fn float_pair_parse(text: &str) -> Result<(f32, f32), String> {
    let Some((first, second)) = text.split_once(',') else {
        return Err(format!("expected 2 comma-separated values, got '{text}'"));
    };
    let first = finite_float_parse(first)?;
    let second = finite_float_parse(second)?;
    Ok((first, second))
}

pub fn finite_float_parse(text: &str) -> Result<f32, String> {
    let value: f32 = text
        .trim()
        .parse()
        .map_err(|_| format!("expected a number, got '{}'", text.trim()))?;
    if !value.is_finite() {
        return Err(format!("expected a finite number, got '{}'", text.trim()));
    }
    Ok(value)
}
