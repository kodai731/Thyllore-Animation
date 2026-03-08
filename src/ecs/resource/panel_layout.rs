const HIERARCHY_WIDTH_DEFAULT: f32 = 250.0;
const INSPECTOR_WIDTH_DEFAULT: f32 = 300.0;
const TIMELINE_HEIGHT_DEFAULT: f32 = 300.0;
const DEBUG_HEIGHT_DEFAULT: f32 = 250.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveSplitter {
    Left,
    Right,
    Upper,
    Lower,
}

#[derive(Clone, Copy, Debug)]
pub struct DragState {
    pub splitter: ActiveSplitter,
    pub start_pos: f32,
    pub start_value: f32,
}

pub struct PanelLayout {
    pub hierarchy_width: f32,
    pub inspector_width: f32,
    pub timeline_height: f32,
    pub debug_height: f32,

    pub drag: Option<DragState>,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            hierarchy_width: HIERARCHY_WIDTH_DEFAULT,
            inspector_width: INSPECTOR_WIDTH_DEFAULT,
            timeline_height: TIMELINE_HEIGHT_DEFAULT,
            debug_height: DEBUG_HEIGHT_DEFAULT,
            drag: None,
        }
    }
}

impl PanelLayout {
    pub fn main_height(&self, display_h: f32) -> f32 {
        display_h - self.timeline_height - self.debug_height
    }

    pub fn viewport_width(&self, display_w: f32) -> f32 {
        display_w - self.hierarchy_width - self.inspector_width
    }

    pub fn default_value_for(splitter: ActiveSplitter) -> f32 {
        match splitter {
            ActiveSplitter::Left => HIERARCHY_WIDTH_DEFAULT,
            ActiveSplitter::Right => INSPECTOR_WIDTH_DEFAULT,
            ActiveSplitter::Upper => TIMELINE_HEIGHT_DEFAULT,
            ActiveSplitter::Lower => DEBUG_HEIGHT_DEFAULT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values_match_hardcoded() {
        let layout = PanelLayout::default();
        assert_eq!(layout.hierarchy_width, 250.0);
        assert_eq!(layout.inspector_width, 300.0);
        assert_eq!(layout.timeline_height, 300.0);
        assert_eq!(layout.debug_height, 250.0);
    }

    #[test]
    fn test_main_height_calculation() {
        let layout = PanelLayout::default();
        let result = layout.main_height(1080.0);
        assert!((result - 530.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_viewport_width_calculation() {
        let layout = PanelLayout::default();
        let result = layout.viewport_width(1920.0);
        assert!((result - 1370.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_drag_state_none_by_default() {
        let layout = PanelLayout::default();
        assert!(layout.drag.is_none());
    }
}
