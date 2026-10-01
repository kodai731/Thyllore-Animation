use crate::expression::components::preset::ExpressionLibrary;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    Neutral,
    Fist,
    HandOpen,
    FingerPoint,
    Victory,
    RockNRoll,
    HandGun,
    ThumbsUp,
}

impl Gesture {
    pub const ALL: [Gesture; 8] = [
        Gesture::Neutral,
        Gesture::Fist,
        Gesture::HandOpen,
        Gesture::FingerPoint,
        Gesture::Victory,
        Gesture::RockNRoll,
        Gesture::HandGun,
        Gesture::ThumbsUp,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Gesture::Neutral => "Neutral",
            Gesture::Fist => "Fist",
            Gesture::HandOpen => "HandOpen",
            Gesture::FingerPoint => "FingerPoint",
            Gesture::Victory => "Victory",
            Gesture::RockNRoll => "RockNRoll",
            Gesture::HandGun => "HandGun",
            Gesture::ThumbsUp => "ThumbsUp",
        }
    }
}

pub fn gesture_template_library() -> ExpressionLibrary {
    let presets = Gesture::ALL
        .iter()
        .map(
            |g| crate::expression::components::preset::ExpressionPreset {
                name: g.name().to_string(),
                weights: std::collections::BTreeMap::new(),
            },
        )
        .collect();

    ExpressionLibrary { presets }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gesture_count() {
        assert_eq!(Gesture::ALL.len(), 8);
    }

    #[test]
    fn test_gesture_names() {
        assert_eq!(Gesture::Neutral.name(), "Neutral");
        assert_eq!(Gesture::Fist.name(), "Fist");
        assert_eq!(Gesture::HandOpen.name(), "HandOpen");
        assert_eq!(Gesture::FingerPoint.name(), "FingerPoint");
        assert_eq!(Gesture::Victory.name(), "Victory");
        assert_eq!(Gesture::RockNRoll.name(), "RockNRoll");
        assert_eq!(Gesture::HandGun.name(), "HandGun");
        assert_eq!(Gesture::ThumbsUp.name(), "ThumbsUp");
    }

    #[test]
    fn test_gesture_template_library() {
        let library = gesture_template_library();

        assert_eq!(library.presets.len(), 8);

        for (i, preset) in library.presets.iter().enumerate() {
            assert_eq!(preset.name, Gesture::ALL[i].name());
            assert!(preset.weights.is_empty());
        }
    }
}
