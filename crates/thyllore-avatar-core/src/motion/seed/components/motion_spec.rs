use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionSide {
    Right,
    Left,
}

/// The slots a sentence resolves to: which motion, which side, how many cycles, how large, how fast.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionSpec {
    pub motion: String,
    pub side: MotionSide,
    pub count: u32,
    pub amount: f32,
    pub speed: f32,
}

impl MotionSpec {
    pub fn new(motion: &str) -> MotionSpec {
        MotionSpec {
            motion: motion.to_string(),
            side: MotionSide::Right,
            count: 1,
            amount: 1.0,
            speed: 1.0,
        }
    }

    /// `<motion>[,side=left|right][,count=<n>][,amount=<f>][,speed=<f>]`
    pub fn parse(text: &str) -> Result<MotionSpec> {
        let mut parts = text.split(',').map(str::trim);
        let motion = parts.next().filter(|m| !m.is_empty()).ok_or_else(|| {
            anyhow::anyhow!("motion spec must start with a motion name, got '{text}'")
        })?;
        let mut spec = MotionSpec::new(motion);

        for part in parts {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("motion option must be key=value, got '{part}'"))?;
            match key {
                "side" => {
                    spec.side = match value {
                        "right" => MotionSide::Right,
                        "left" => MotionSide::Left,
                        other => bail!("side must be left or right, got '{other}'"),
                    }
                }
                "count" => {
                    spec.count = value.parse().ok().filter(|c| *c >= 1).ok_or_else(|| {
                        anyhow::anyhow!("count must be an integer >= 1, got '{value}'")
                    })?
                }
                "amount" => spec.amount = parse_positive(value, "amount")?,
                "speed" => spec.speed = parse_positive(value, "speed")?,
                other => {
                    bail!("unknown motion option '{other}' (expected side, count, amount, speed)")
                }
            }
        }
        Ok(spec)
    }
}

impl std::fmt::Display for MotionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.motion)?;
        if self.side == MotionSide::Left {
            write!(f, ",side=left")?;
        }
        if self.count != 1 {
            write!(f, ",count={}", self.count)?;
        }
        if self.amount != 1.0 {
            write!(f, ",amount={}", self.amount)?;
        }
        if self.speed != 1.0 {
            write!(f, ",speed={}", self.speed)?;
        }
        Ok(())
    }
}

fn parse_positive(value: &str, name: &str) -> Result<f32> {
    value
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| anyhow::anyhow!("{name} must be a positive number, got '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_options() {
        let spec = MotionSpec::parse("punch,side=left,count=2,amount=0.8,speed=1.5").unwrap();
        assert_eq!(spec.motion, "punch");
        assert_eq!(spec.side, MotionSide::Left);
        assert_eq!(spec.count, 2);
        assert_eq!(spec.amount, 0.8);
        assert_eq!(spec.speed, 1.5);
    }

    #[test]
    fn defaults_are_right_once_normal() {
        assert_eq!(MotionSpec::parse("bow").unwrap(), MotionSpec::new("bow"));
    }

    #[test]
    fn display_round_trips_through_parse() {
        for text in [
            "bow",
            "punch,side=left,count=2,amount=0.8,speed=1.5",
            "wave,count=3",
        ] {
            let spec = MotionSpec::parse(text).unwrap();
            assert_eq!(spec.to_string(), text);
            assert_eq!(MotionSpec::parse(&spec.to_string()).unwrap(), spec);
        }
    }

    #[test]
    fn rejects_bad_options() {
        for text in [
            "",
            "punch,side=up",
            "punch,count=0",
            "punch,speed=-1",
            "punch,tempo=2",
        ] {
            assert!(MotionSpec::parse(text).is_err(), "{text}");
        }
    }
}
