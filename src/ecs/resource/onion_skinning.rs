use crate::vulkanr::data::Vertex;

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

#[derive(Clone, Debug)]
pub struct GhostMeshData {
    pub vertices: Vec<Vertex>,
    pub tint_color: [f32; 3],
    pub opacity: f32,
    pub mesh_index: usize,
}

pub struct OnionSkinningResult {
    pub ghost_meshes: Vec<GhostMeshData>,
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
}
