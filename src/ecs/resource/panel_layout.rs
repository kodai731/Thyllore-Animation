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

    pub fn constrain_to_display(&mut self, display_w: f32, display_h: f32) {
        debug_assert!(display_w > 0.0, "display_w must be positive");
        debug_assert!(display_h > 0.0, "display_h must be positive");
        const HIERARCHY_WIDTH_MIN: f32 = 150.0;
        const HIERARCHY_WIDTH_MAX: f32 = 500.0;
        const INSPECTOR_WIDTH_MIN: f32 = 200.0;
        const INSPECTOR_WIDTH_MAX: f32 = 500.0;
        const TIMELINE_HEIGHT_MIN: f32 = 100.0;
        const TIMELINE_HEIGHT_MAX: f32 = 600.0;
        const DEBUG_HEIGHT_MIN: f32 = 100.0;
        const DEBUG_HEIGHT_MAX: f32 = 500.0;
        const VIEWPORT_MIN_WIDTH: f32 = 200.0;
        const VIEWPORT_MIN_HEIGHT: f32 = 100.0;

        self.hierarchy_width = self
            .hierarchy_width
            .clamp(HIERARCHY_WIDTH_MIN, HIERARCHY_WIDTH_MAX);
        self.inspector_width = self
            .inspector_width
            .clamp(INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX);
        self.timeline_height = self
            .timeline_height
            .clamp(TIMELINE_HEIGHT_MIN, TIMELINE_HEIGHT_MAX);
        self.debug_height = self.debug_height.clamp(DEBUG_HEIGHT_MIN, DEBUG_HEIGHT_MAX);

        let max_side_total = display_w - VIEWPORT_MIN_WIDTH;
        if self.hierarchy_width + self.inspector_width > max_side_total {
            let ratio = self.hierarchy_width / (self.hierarchy_width + self.inspector_width);
            self.hierarchy_width =
                (max_side_total * ratio).clamp(HIERARCHY_WIDTH_MIN, HIERARCHY_WIDTH_MAX);
            self.inspector_width = (max_side_total - self.hierarchy_width)
                .clamp(INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX);
        }

        let max_bottom_total = display_h - VIEWPORT_MIN_HEIGHT;
        if self.timeline_height + self.debug_height > max_bottom_total {
            let ratio = self.timeline_height / (self.timeline_height + self.debug_height);
            self.timeline_height =
                (max_bottom_total * ratio).clamp(TIMELINE_HEIGHT_MIN, TIMELINE_HEIGHT_MAX);
            self.debug_height =
                (max_bottom_total - self.timeline_height).clamp(DEBUG_HEIGHT_MIN, DEBUG_HEIGHT_MAX);
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

    #[test]
    fn test_constrain_prevents_viewport_too_small() {
        let mut layout = PanelLayout::default();
        layout.hierarchy_width = 800.0;
        layout.inspector_width = 800.0;
        layout.constrain_to_display(1920.0, 1080.0);

        let viewport_w = layout.viewport_width(1920.0);
        assert!(viewport_w >= 200.0);
    }

    #[test]
    fn test_constrain_respects_min_max() {
        let mut layout = PanelLayout::default();
        layout.hierarchy_width = 10.0;
        layout.inspector_width = 10.0;
        layout.timeline_height = 10.0;
        layout.debug_height = 10.0;
        layout.constrain_to_display(1920.0, 1080.0);

        assert!(layout.hierarchy_width >= 150.0);
        assert!(layout.inspector_width >= 200.0);
        assert!(layout.timeline_height >= 100.0);
        assert!(layout.debug_height >= 100.0);
    }

    #[test]
    fn test_constrain_vertical_constraints() {
        let mut layout = PanelLayout::default();
        layout.timeline_height = 600.0;
        layout.debug_height = 500.0;
        layout.constrain_to_display(1920.0, 1080.0);

        let main_h = layout.main_height(1080.0);
        assert!(main_h >= 100.0);
    }
}
