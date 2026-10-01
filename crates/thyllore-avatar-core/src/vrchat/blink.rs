use crate::expression::systems::side::channel_side;

#[derive(Clone, Debug, Default)]
pub struct BlinkCandidates {
    pub both: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

fn is_blink_channel(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.contains("blink") {
        return true;
    }
    if lower.contains("close") && lower.contains("eye") {
        return true;
    }
    false
}

pub fn find_blink_candidates(channel_names: &[String]) -> BlinkCandidates {
    let mut candidates = BlinkCandidates::default();

    for name in channel_names {
        if !is_blink_channel(name) {
            continue;
        }

        match channel_side(name) {
            Some(crate::expression::components::side::Side::Left) => {
                if candidates.left.is_none() {
                    candidates.left = Some(name.clone());
                }
            }
            Some(crate::expression::components::side::Side::Right) => {
                if candidates.right.is_none() {
                    candidates.right = Some(name.clone());
                }
            }
            None => {
                if candidates.both.is_none() {
                    candidates.both = Some(name.clone());
                }
            }
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blink_channel_contains_blink() {
        assert!(is_blink_channel("eye_blink"));
        assert!(is_blink_channel("Blink"));
        assert!(is_blink_channel("left_blink_L"));
    }

    #[test]
    fn test_is_blink_channel_close_and_eye() {
        assert!(is_blink_channel("eye_close"));
        assert!(is_blink_channel("close_eye"));
        assert!(is_blink_channel("Eye_Close_L"));
    }

    #[test]
    fn test_is_blink_channel_false() {
        assert!(!is_blink_channel("eye_open"));
        assert!(!is_blink_channel("close_mouth"));
    }

    #[test]
    fn test_find_blink_candidates_empty() {
        let candidates = find_blink_candidates(&[]);
        assert!(candidates.both.is_none());
        assert!(candidates.left.is_none());
        assert!(candidates.right.is_none());
    }

    #[test]
    fn test_find_blink_candidates_both_only() {
        let channels: Vec<String> = vec!["eye_blink".to_string()];
        let candidates = find_blink_candidates(&channels);
        assert_eq!(candidates.both, Some("eye_blink".to_string()));
        assert!(candidates.left.is_none());
        assert!(candidates.right.is_none());
    }

    #[test]
    fn test_find_blink_candidates_sided() {
        let channels: Vec<String> = vec!["eye_blink_L".to_string(), "eye_blink_R".to_string()];
        let candidates = find_blink_candidates(&channels);
        assert!(candidates.both.is_none());
        assert_eq!(candidates.left, Some("eye_blink_L".to_string()));
        assert_eq!(candidates.right, Some("eye_blink_R".to_string()));
    }

    #[test]
    fn test_find_blink_candidates_close_eye() {
        let channels: Vec<String> = vec!["Eye_Close_L".to_string(), "Eye_Close_R".to_string()];
        let candidates = find_blink_candidates(&channels);
        assert!(candidates.both.is_none());
        assert_eq!(candidates.left, Some("Eye_Close_L".to_string()));
        assert_eq!(candidates.right, Some("Eye_Close_R".to_string()));
    }

    #[test]
    fn test_find_blink_candidates_mixed() {
        let channels: Vec<String> = vec![
            "eye_blink".to_string(),
            "eye_blink_L".to_string(),
            "eye_blink_R".to_string(),
        ];
        let candidates = find_blink_candidates(&channels);
        assert_eq!(candidates.both, Some("eye_blink".to_string()));
        assert_eq!(candidates.left, Some("eye_blink_L".to_string()));
        assert_eq!(candidates.right, Some("eye_blink_R".to_string()));
    }

    #[test]
    fn test_find_blink_candidates_ignores_non_blink() {
        let channels: Vec<String> = vec![
            "mouth_smile".to_string(),
            "eye_open".to_string(),
            "eye_blink_L".to_string(),
        ];
        let candidates = find_blink_candidates(&channels);
        assert!(candidates.both.is_none());
        assert_eq!(candidates.left, Some("eye_blink_L".to_string()));
        assert!(candidates.right.is_none());
    }

    #[test]
    fn test_find_blink_candidates_first_match_only() {
        let channels: Vec<String> = vec!["eye_blink_L".to_string(), "eye_close_L".to_string()];
        let candidates = find_blink_candidates(&channels);
        assert_eq!(candidates.left, Some("eye_blink_L".to_string()));
    }
}
