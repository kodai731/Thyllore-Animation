use anyhow::{bail, Context, Result};

pub fn flag_value_resolve_from_args(args: &[String], flag: &str) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1).filter(|v| !v.starts_with("--")) else {
        bail!("{flag} requires a value");
    };
    Ok(Some(value.clone()))
}

/// Parses every `KEY=VALUE` given as `<flag> KEY=VALUE` or `<flag>=KEY=VALUE`, rejecting keys
/// outside `valid_keys`.
pub fn scalar_set_resolve_from_args(
    args: &[String],
    flag: &str,
    valid_keys: &[&'static str],
) -> Result<Vec<(String, f32)>> {
    let flag_name = flag.trim_start_matches('-');

    let mut pairs: Vec<(String, f32)> = Vec::new();
    for i in 0..args.len() {
        let payload = if args[i] == flag {
            if i + 1 >= args.len() {
                bail!("{} requires a value after it", flag);
            }
            args[i + 1].clone()
        } else if let Some(rest) = args[i].strip_prefix(flag) {
            rest.trim_start_matches('=').trim().to_string()
        } else {
            continue;
        };

        let parts: Vec<&str> = payload.splitn(2, '=').collect();
        if parts.len() != 2 {
            bail!(
                "{} value must be KEY=VALUE format, got '{}'",
                flag_name,
                payload
            );
        }
        let key = parts[0].trim().to_string();
        let value_str = parts[1].trim();
        let value: f32 = value_str.parse().context(format!(
            "{} value must be a number, got '{}'",
            flag_name, value_str
        ))?;

        if !valid_keys.contains(&key.as_str()) {
            bail!(
                "unknown {} key '{}'. Valid keys: {}",
                flag_name,
                key,
                valid_keys.join(", ")
            );
        }

        pairs.push((key, value));
    }
    Ok(pairs)
}
