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

pub fn nonnegative_finite_float_parse(text: &str) -> Result<f32, String> {
    let value = finite_float_parse(text)?;
    if value < 0.0 {
        return Err(format!(
            "expected a non-negative number, got '{}'",
            text.trim()
        ));
    }
    Ok(value)
}

pub fn required_split<'a>(
    text: &'a str,
    separator: char,
    usage: &str,
) -> Result<(&'a str, &'a str), String> {
    text.split_once(separator)
        .ok_or_else(|| format!("expected '{}', got '{}' ({})", separator, text, usage))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_value_resolve_from_args_missing_flag() {
        let args: Vec<String> = vec!["--other".into(), "val".into()];
        assert_eq!(flag_value_resolve_from_args(&args, "--foo").unwrap(), None);
    }

    #[test]
    fn flag_value_resolve_from_args_present() {
        let args: Vec<String> = vec!["--foo".into(), "bar".into()];
        assert_eq!(
            flag_value_resolve_from_args(&args, "--foo").unwrap(),
            Some("bar".into())
        );
    }

    #[test]
    fn scalar_assignment_parse_valid() {
        let (key, value) = scalar_assignment_parse("intensity=2.5", &["intensity"]).unwrap();
        assert_eq!(key, "intensity");
        assert!((value - 2.5).abs() < 1e-6);
    }

    #[test]
    fn scalar_assignment_parse_unknown_key() {
        let err = scalar_assignment_parse("unknown=1.0", &["intensity"]).unwrap_err();
        assert!(err.contains("unknown key"));
    }

    #[test]
    fn float_pair_parse_valid() {
        let (a, b) = float_pair_parse("1.5,2.5").unwrap();
        assert!((a - 1.5).abs() < 1e-6);
        assert!((b - 2.5).abs() < 1e-6);
    }

    #[test]
    fn finite_float_parse_valid() {
        let v = finite_float_parse("3.14").unwrap();
        assert!((v - 3.14).abs() < 1e-6);
    }

    #[test]
    fn finite_float_parse_infinite_rejected() {
        let err = finite_float_parse("inf").unwrap_err();
        assert!(err.contains("finite"));
    }

    #[test]
    fn nonnegative_finite_float_parse_valid() {
        let v = nonnegative_finite_float_parse("2.5").unwrap();
        assert!((v - 2.5).abs() < 1e-6);
    }

    #[test]
    fn nonnegative_finite_float_parse_zero() {
        let v = nonnegative_finite_float_parse("0").unwrap();
        assert!((v - 0.0).abs() < 1e-6);
    }

    #[test]
    fn nonnegative_finite_float_parse_negative_rejected() {
        let err = nonnegative_finite_float_parse("-1.0").unwrap_err();
        assert!(err.contains("non-negative"));
    }

    #[test]
    fn nonnegative_finite_float_parse_infinite_rejected() {
        let err = nonnegative_finite_float_parse("inf").unwrap_err();
        assert!(err.contains("finite"));
    }

    #[test]
    fn required_split_valid() {
        let (left, right) = required_split("a=b", '=', "usage").unwrap();
        assert_eq!(left, "a");
        assert_eq!(right, "b");
    }

    #[test]
    fn required_split_missing_separator() {
        let err = required_split("abc", '=', "usage").unwrap_err();
        assert!(err.contains("usage"));
    }
}
