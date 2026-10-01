use crate::expression::components::side::{MarkerPosition, Side, SIDE_MARKERS};

pub fn find_mirror_channel(name: &str) -> Option<String> {
    for marker in SIDE_MARKERS {
        match marker.position {
            MarkerPosition::Suffix => {
                if let Some(stem) = name.strip_suffix(marker.left) {
                    return Some(format!("{}{}", stem, marker.right));
                }
                if let Some(stem) = name.strip_suffix(marker.right) {
                    return Some(format!("{}{}", stem, marker.left));
                }
            }
            MarkerPosition::Prefix => {
                if let Some(rest) = name.strip_prefix(marker.left) {
                    return Some(format!("{}{}", marker.right, rest));
                }
                if let Some(rest) = name.strip_prefix(marker.right) {
                    return Some(format!("{}{}", marker.left, rest));
                }
            }
        }
    }
    None
}

pub fn channel_side(name: &str) -> Option<Side> {
    for marker in SIDE_MARKERS {
        match marker.position {
            MarkerPosition::Suffix => {
                if name.ends_with(marker.left) {
                    return Some(Side::Left);
                }
                if name.ends_with(marker.right) {
                    return Some(Side::Right);
                }
            }
            MarkerPosition::Prefix => {
                if name.starts_with(marker.left) {
                    return Some(Side::Left);
                }
                if name.starts_with(marker.right) {
                    return Some(Side::Right);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_mirror_channel_suffix_upper() {
        assert_eq!(
            find_mirror_channel("eye_angry_L"),
            Some("eye_angry_R".to_string())
        );
        assert_eq!(
            find_mirror_channel("eye_angry_R"),
            Some("eye_angry_L".to_string())
        );
    }

    #[test]
    fn test_find_mirror_channel_suffix_dot() {
        assert_eq!(
            find_mirror_channel("Upper_arm.L"),
            Some("Upper_arm.R".to_string())
        );
        assert_eq!(
            find_mirror_channel("Upper_arm.R"),
            Some("Upper_arm.L".to_string())
        );
    }

    #[test]
    fn test_find_mirror_channel_suffix_lower() {
        assert_eq!(
            find_mirror_channel("eye_angry_l"),
            Some("eye_angry_r".to_string())
        );
        assert_eq!(
            find_mirror_channel("eye_angry_r"),
            Some("eye_angry_l".to_string())
        );
    }

    #[test]
    fn test_find_mirror_channel_prefix() {
        assert_eq!(
            find_mirror_channel("Left_eye"),
            Some("Right_eye".to_string())
        );
        assert_eq!(
            find_mirror_channel("Right_eye"),
            Some("Left_eye".to_string())
        );
    }

    #[test]
    fn test_find_mirror_channel_none() {
        assert_eq!(find_mirror_channel("eye_angry"), None);
        assert_eq!(find_mirror_channel("mouth_smile"), None);
    }

    #[test]
    fn test_channel_side_suffix_upper() {
        assert_eq!(channel_side("eye_angry_L"), Some(Side::Left));
        assert_eq!(channel_side("eye_angry_R"), Some(Side::Right));
    }

    #[test]
    fn test_channel_side_suffix_dot() {
        assert_eq!(channel_side("Upper_arm.L"), Some(Side::Left));
        assert_eq!(channel_side("Upper_arm.R"), Some(Side::Right));
    }

    #[test]
    fn test_channel_side_suffix_lower() {
        assert_eq!(channel_side("eye_angry_l"), Some(Side::Left));
        assert_eq!(channel_side("eye_angry_r"), Some(Side::Right));
    }

    #[test]
    fn test_channel_side_prefix() {
        assert_eq!(channel_side("Left_eye"), Some(Side::Left));
        assert_eq!(channel_side("Right_eye"), Some(Side::Right));
    }

    #[test]
    fn test_channel_side_none() {
        assert_eq!(channel_side("eye_angry"), None);
        assert_eq!(channel_side("mouth_smile"), None);
    }

    #[test]
    fn test_mirror_is_involutive() {
        let names = [
            "eye_angry_L",
            "eye_angry_R",
            "Upper_arm.L",
            "Upper_arm.R",
            "eye_angry_l",
            "eye_angry_r",
            "Left_eye",
            "Right_eye",
        ];
        for name in names {
            let mirror = find_mirror_channel(name).unwrap();
            let back = find_mirror_channel(&mirror).unwrap();
            assert_eq!(back, name, "mirror of {} -> {} -> {}", name, mirror, back);
        }
    }
}
