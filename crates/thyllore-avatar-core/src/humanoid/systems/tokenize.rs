use crate::expression::components::side::{MarkerPosition, Side, SIDE_MARKERS};
use crate::humanoid::components::tokens::BoneNameTokens;

pub fn tokenize_bone_name(name: &str) -> BoneNameTokens {
    let (stripped, side) = strip_side_markers(strip_namespace(name));
    let tokens = split_into_tokens(&stripped);
    BoneNameTokens { tokens, side }
}

fn strip_namespace(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn strip_side_markers(name: &str) -> (String, Option<Side>) {
    for marker in SIDE_MARKERS {
        if marker.position == MarkerPosition::Suffix {
            if let Some(stripped) = name.strip_suffix(marker.left) {
                return (stripped.to_string(), Some(Side::Left));
            }
            if let Some(stripped) = name.strip_suffix(marker.right) {
                return (stripped.to_string(), Some(Side::Right));
            }
        } else if marker.position == MarkerPosition::Prefix {
            if let Some(stripped) = name.strip_prefix(marker.left) {
                return (stripped.to_string(), Some(Side::Left));
            }
            if let Some(stripped) = name.strip_prefix(marker.right) {
                return (stripped.to_string(), Some(Side::Right));
            }
        }
    }

    for (prefix, side) in [("Left", Side::Left), ("Right", Side::Right)] {
        if let Some(stripped) = name
            .strip_prefix(prefix)
            .filter(|rest| rest.starts_with(char::is_uppercase))
        {
            return (stripped.to_string(), Some(side));
        }
    }

    let mut parts = split_on_delimiters(name);
    let side_part = parts
        .iter()
        .enumerate()
        .find_map(|(index, part)| side_of_word(part).map(|side| (index, side)));
    match side_part {
        Some((index, side)) if parts.len() >= 2 => {
            parts.remove(index);
            (parts.join("_"), Some(side))
        }
        _ => (name.to_string(), None),
    }
}

fn side_of_word(word: &str) -> Option<Side> {
    match word.to_lowercase().as_str() {
        "l" | "left" => Some(Side::Left),
        "r" | "right" => Some(Side::Right),
        _ => None,
    }
}

fn split_on_delimiters(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in s.chars() {
        if ch == '_' || ch == '.' || ch == ' ' || ch == '-' {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn split_into_tokens(s: &str) -> Vec<String> {
    let parts = split_on_delimiters(s);
    let mut tokens = Vec::new();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let camel_tokens = split_camel_case(&part);
        for token in camel_tokens {
            let lower = token.to_lowercase();
            if !lower.is_empty() {
                tokens.push(lower);
            }
        }
    }
    tokens
}

fn split_camel_case(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let ch = chars[i];
        if ch.is_uppercase() && !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
        current.push(ch);

        if i + 1 < len {
            let next = chars[i + 1];
            if ch.is_uppercase() && next.is_lowercase() && current.len() >= 2 {
                let last = current.chars().next_back().unwrap();
                current.pop();
                tokens.push(current.clone());
                current.clear();
                current.push(last);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::components::side::Side;

    #[test]
    fn test_blender_style_with_suffix_markers() {
        let tokens = tokenize_bone_name("Upper_arm.L");
        assert_eq!(tokens.tokens, vec!["upper", "arm"]);
        assert_eq!(tokens.side, Some(Side::Left));

        let tokens = tokenize_bone_name("Upper_arm.R");
        assert_eq!(tokens.tokens, vec!["upper", "arm"]);
        assert_eq!(tokens.side, Some(Side::Right));
    }

    #[test]
    fn test_unity_style_with_prefix() {
        let tokens = tokenize_bone_name("LeftUpperArm");
        assert_eq!(tokens.tokens, vec!["upper", "arm"]);
        assert_eq!(tokens.side, Some(Side::Left));

        let tokens = tokenize_bone_name("RightUpperArm");
        assert_eq!(tokens.tokens, vec!["upper", "arm"]);
        assert_eq!(tokens.side, Some(Side::Right));
    }

    #[test]
    fn test_j_style_with_prefix_and_suffix() {
        let tokens = tokenize_bone_name("J_Bip_L_UpperArm");
        assert_eq!(tokens.tokens, vec!["j", "bip", "upper", "arm"]);
        assert_eq!(tokens.side, Some(Side::Left));
    }

    #[test]
    fn test_center_bones() {
        let tokens = tokenize_bone_name("Hips");
        assert_eq!(tokens.tokens, vec!["hips"]);
        assert_eq!(tokens.side, None);

        let tokens = tokenize_bone_name("Spine");
        assert_eq!(tokens.tokens, vec!["spine"]);
        assert_eq!(tokens.side, None);

        let tokens = tokenize_bone_name("Head");
        assert_eq!(tokens.tokens, vec!["head"]);
        assert_eq!(tokens.side, None);
    }

    #[test]
    fn test_camel_case_splitting() {
        let tokens = tokenize_bone_name("LowerArm");
        assert_eq!(tokens.tokens, vec!["lower", "arm"]);

        let tokens = tokenize_bone_name("UpperLeg");
        assert_eq!(tokens.tokens, vec!["upper", "leg"]);
    }

    #[test]
    fn test_underscore_delimiter() {
        let tokens = tokenize_bone_name("left_hand");
        assert_eq!(tokens.tokens, vec!["hand"]);
        assert_eq!(tokens.side, Some(Side::Left));
    }

    #[test]
    fn test_dot_delimiter() {
        let tokens = tokenize_bone_name("Hand.L");
        assert_eq!(tokens.tokens, vec!["hand"]);
        assert_eq!(tokens.side, Some(Side::Left));
    }

    #[test]
    fn test_trailing_l_r_tokens() {
        let tokens = tokenize_bone_name("UpperArm l");
        assert_eq!(tokens.tokens, vec!["upper", "arm"]);
        assert_eq!(tokens.side, Some(Side::Left));
    }

    #[test]
    fn test_namespace_prefix() {
        let tokens = tokenize_bone_name("mixamorig:LeftForeArm");
        assert_eq!(tokens.tokens, vec!["fore", "arm"]);
        assert_eq!(tokens.side, Some(Side::Left));
    }
}
