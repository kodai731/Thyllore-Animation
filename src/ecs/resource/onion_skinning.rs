#[derive(Clone, Debug)]
pub struct OnionSkinningConfig {
    pub enabled: bool,
    pub past_count: u32,
    pub future_count: u32,
    pub frame_step: f32,
    pub past_color: [f32; 3],
    pub future_color: [f32; 3],
    pub opacity: f32,
}

impl OnionSkinningConfig {
    pub fn total_ghost_count(&self) -> u32 {
        if self.enabled {
            self.past_count + self.future_count
        } else {
            0
        }
    }

    pub fn ghost_time_offsets(&self) -> Vec<GhostFrameInfo> {
        if !self.enabled {
            return Vec::new();
        }

        let mut offsets = Vec::new();

        for i in 1..=self.past_count {
            let distance = i as f32;
            let opacity = self.opacity * (1.0 - (distance - 1.0) / self.past_count.max(1) as f32);
            offsets.push(GhostFrameInfo {
                time_offset: -(i as f32) * self.frame_step,
                tint_color: self.past_color,
                opacity,
            });
        }

        for i in 1..=self.future_count {
            let distance = i as f32;
            let opacity =
                self.opacity * (1.0 - (distance - 1.0) / self.future_count.max(1) as f32);
            offsets.push(GhostFrameInfo {
                time_offset: i as f32 * self.frame_step,
                tint_color: self.future_color,
                opacity,
            });
        }

        offsets
    }
}

impl Default for OnionSkinningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            past_count: 2,
            future_count: 2,
            frame_step: 1.0 / 30.0,
            past_color: [0.2, 0.4, 1.0],
            future_color: [1.0, 0.4, 0.2],
            opacity: 0.4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GhostFrameInfo {
    pub time_offset: f32,
    pub tint_color: [f32; 3],
    pub opacity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OnionSkinningConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.past_count, 2);
        assert_eq!(config.future_count, 2);
        assert!((config.frame_step - 1.0 / 30.0).abs() < f32::EPSILON);
        assert!((config.opacity - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_total_ghost_count_disabled() {
        let config = OnionSkinningConfig::default();
        assert_eq!(config.total_ghost_count(), 0);
    }

    #[test]
    fn test_total_ghost_count_enabled() {
        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        assert_eq!(config.total_ghost_count(), 4);
    }

    #[test]
    fn test_ghost_time_offsets_disabled() {
        let config = OnionSkinningConfig::default();
        assert!(config.ghost_time_offsets().is_empty());
    }

    #[test]
    fn test_ghost_time_offsets_enabled() {
        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        config.past_count = 2;
        config.future_count = 1;

        let offsets = config.ghost_time_offsets();
        assert_eq!(offsets.len(), 3);

        assert!(offsets[0].time_offset < 0.0);
        assert!(offsets[1].time_offset < 0.0);
        assert!(offsets[2].time_offset > 0.0);

        assert!(offsets[0].opacity > 0.0);
        assert!(offsets[1].opacity > 0.0);
        assert!(offsets[2].opacity > 0.0);

        assert_eq!(offsets[0].tint_color, config.past_color);
        assert_eq!(offsets[2].tint_color, config.future_color);
    }

    #[test]
    fn test_ghost_opacity_falloff() {
        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        config.past_count = 3;
        config.future_count = 0;
        config.opacity = 0.6;

        let offsets = config.ghost_time_offsets();
        assert_eq!(offsets.len(), 3);

        assert!(offsets[0].opacity >= offsets[1].opacity);
        assert!(offsets[1].opacity >= offsets[2].opacity);
    }
}
