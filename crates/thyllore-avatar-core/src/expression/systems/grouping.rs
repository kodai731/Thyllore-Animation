use crate::expression::components::grouping::{ChannelGroup, ExpressionGrouping};

pub fn group_channels(names: &[String], grouping: &ExpressionGrouping) -> Vec<ChannelGroup> {
    let mut groups: Vec<ChannelGroup> = grouping
        .groups
        .iter()
        .map(|rule| ChannelGroup {
            name: rule.name.clone(),
            channel_indices: Vec::new(),
            collapsed: rule.collapsed,
            exclude_from_reset: rule.exclude_from_reset,
        })
        .collect();

    let mut other = ChannelGroup {
        name: "Other".to_string(),
        channel_indices: Vec::new(),
        collapsed: false,
        exclude_from_reset: false,
    };

    for (i, name) in names.iter().enumerate() {
        let best = find_best_match(name, grouping);
        match best {
            Some(rule_index) => groups[rule_index].channel_indices.push(i),
            None => other.channel_indices.push(i),
        }
    }

    groups.push(other);
    groups
}

fn find_best_match(name: &str, grouping: &ExpressionGrouping) -> Option<usize> {
    let mut best_index: Option<usize> = None;
    let mut best_length: usize = 0;

    for (rule_index, rule) in grouping.groups.iter().enumerate() {
        for prefix in &rule.prefixes {
            if matches_prefix(name, prefix) && prefix.len() > best_length {
                best_index = Some(rule_index);
                best_length = prefix.len();
            }
        }
    }

    best_index
}

fn matches_prefix(name: &str, prefix: &str) -> bool {
    if name == prefix {
        return true;
    }
    if let Some(suffix) = name.strip_prefix(prefix) {
        suffix.starts_with('_') || suffix.starts_with('.')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_grouping() -> ExpressionGrouping {
        ExpressionGrouping::default()
    }

    #[test]
    fn test_matches_prefix_exact() {
        assert!(matches_prefix("eye", "eye"));
        assert!(matches_prefix("Shrink", "Shrink"));
    }

    #[test]
    fn test_matches_prefix_underscore_continuation() {
        assert!(matches_prefix("eye_angry", "eye"));
        assert!(matches_prefix("eyebrow_raise", "eyebrow"));
    }

    #[test]
    fn test_matches_prefix_dot_continuation() {
        assert!(matches_prefix("eye.left", "eye"));
    }

    #[test]
    fn test_matches_prefix_no_match() {
        assert!(!matches_prefix("eyes", "eye"));
        assert!(!matches_prefix("eyebrows", "eyebrow"));
        assert!(!matches_prefix("eyeof", "eye"));
    }

    #[test]
    fn test_longest_prefix_wins() {
        let names = vec!["eyebrow_raise".to_string(), "eye_angry".to_string()];
        let groups = group_channels(&names, &default_grouping());

        let eyebrow_group = groups.iter().find(|g| g.name == "Eyebrow").unwrap();
        assert_eq!(eyebrow_group.channel_indices, vec![0]);

        let eye_group = groups.iter().find(|g| g.name == "Eye").unwrap();
        assert_eq!(eye_group.channel_indices, vec![1]);
    }

    #[test]
    fn test_implicit_other_group() {
        let names = vec!["unknown_channel".to_string()];
        let groups = group_channels(&names, &default_grouping());

        let other = groups.iter().find(|g| g.name == "Other").unwrap();
        assert_eq!(other.channel_indices, vec![0]);
    }

    #[test]
    fn test_body_shape_flags() {
        let names = vec!["Shrink".to_string()];
        let groups = group_channels(&names, &default_grouping());

        let body_shape = groups.iter().find(|g| g.name == "BodyShape").unwrap();
        assert!(body_shape.collapsed);
        assert!(body_shape.exclude_from_reset);
    }

    #[test]
    fn test_viseme_prefix() {
        let names = vec!["vrc.v_a".to_string(), "vrc.v_e".to_string()];
        let groups = group_channels(&names, &default_grouping());

        let viseme = groups.iter().find(|g| g.name == "Viseme").unwrap();
        assert_eq!(viseme.channel_indices, vec![0, 1]);
    }

    #[test]
    fn test_case_sensitive() {
        assert!(!matches_prefix("Eye_angry", "eye"));
    }

    #[test]
    fn test_multiple_channels_per_group() {
        let names = vec![
            "eye_angry".to_string(),
            "eye_happy".to_string(),
            "mouth_smile".to_string(),
            "mouth_frown".to_string(),
        ];
        let groups = group_channels(&names, &default_grouping());

        let eye = groups.iter().find(|g| g.name == "Eye").unwrap();
        assert_eq!(eye.channel_indices, vec![0, 1]);

        let mouth = groups.iter().find(|g| g.name == "Mouth").unwrap();
        assert_eq!(mouth.channel_indices, vec![2, 3]);
    }

    #[test]
    fn test_empty_names() {
        let names: Vec<String> = vec![];
        let groups = group_channels(&names, &default_grouping());

        for group in &groups {
            assert!(
                group.channel_indices.is_empty(),
                "group {} should be empty",
                group.name
            );
        }
    }

    #[test]
    fn test_all_channel_indices_accounted_for() {
        let names = vec![
            "eye_angry".to_string(),
            "eyebrow_raise".to_string(),
            "mouth_smile".to_string(),
            "cheek_puff".to_string(),
            "vrc.v_a".to_string(),
            "Shrink".to_string(),
            "unknown".to_string(),
        ];
        let groups = group_channels(&names, &default_grouping());

        let mut all_indices: Vec<usize> = groups
            .iter()
            .flat_map(|g| g.channel_indices.clone())
            .collect();
        all_indices.sort();
        assert_eq!(all_indices, vec![0, 1, 2, 3, 4, 5, 6]);
    }
}
