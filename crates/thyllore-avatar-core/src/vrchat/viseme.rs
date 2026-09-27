pub const VISEME_CHANNELS: [(&str, &str); 15] = [
    ("sil", "vrc.v_sil"),
    ("pp", "vrc.v_pp"),
    ("ff", "vrc.v_ff"),
    ("th", "vrc.v_th"),
    ("dd", "vrc.v_dd"),
    ("kk", "vrc.v_kk"),
    ("ch", "vrc.v_ch"),
    ("ss", "vrc.v_ss"),
    ("nn", "vrc.v_nn"),
    ("rr", "vrc.v_rr"),
    ("aa", "vrc.v_aa"),
    ("e", "vrc.v_e"),
    ("ih", "vrc.v_ih"),
    ("oh", "vrc.v_oh"),
    ("ou", "vrc.v_ou"),
];

pub fn find_missing_visemes(channel_names: &[String]) -> Vec<&'static str> {
    VISEME_CHANNELS
        .iter()
        .filter(|(_, channel)| !channel_names.contains(&channel.to_string()))
        .map(|(key, _)| *key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viseeme_channels_count() {
        assert_eq!(VISEME_CHANNELS.len(), 15);
    }

    #[test]
    fn test_find_missing_visemes_empty() {
        let all: Vec<String> = VISEME_CHANNELS.iter().map(|(_, c)| c.to_string()).collect();
        let missing = find_missing_visemes(&all);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_find_missing_visemes_none() {
        let missing = find_missing_visemes(&[]);
        assert_eq!(missing.len(), 15);
        assert!(missing.contains(&"sil"));
        assert!(missing.contains(&"ou"));
    }

    #[test]
    fn test_find_missing_visemes_partial() {
        let channels: Vec<String> = vec!["vrc.v_sil".to_string(), "vrc.v_aa".to_string()];
        let missing = find_missing_visemes(&channels);
        assert!(!missing.contains(&"sil"));
        assert!(!missing.contains(&"aa"));
        assert!(missing.contains(&"pp"));
        assert_eq!(missing.len(), 13);
    }

    #[test]
    fn test_channel_format() {
        for (key, channel) in &VISEME_CHANNELS {
            assert!(
                channel.starts_with("vrc.v_"),
                "channel {} does not start with vrc.v_",
                channel
            );
            assert_eq!(&channel[6..], *key, "channel suffix mismatch for {}", key);
        }
    }
}
