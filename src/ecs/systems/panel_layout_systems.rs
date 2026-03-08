use crate::ecs::resource::PanelLayout;

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

pub fn panel_layout_clamp_to_display(layout: &mut PanelLayout, display_w: f32, display_h: f32) {
    layout.hierarchy_width = layout
        .hierarchy_width
        .clamp(HIERARCHY_WIDTH_MIN, HIERARCHY_WIDTH_MAX);
    layout.inspector_width = layout
        .inspector_width
        .clamp(INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX);
    layout.timeline_height = layout
        .timeline_height
        .clamp(TIMELINE_HEIGHT_MIN, TIMELINE_HEIGHT_MAX);
    layout.debug_height = layout
        .debug_height
        .clamp(DEBUG_HEIGHT_MIN, DEBUG_HEIGHT_MAX);

    let max_side_total = display_w - VIEWPORT_MIN_WIDTH;
    if layout.hierarchy_width + layout.inspector_width > max_side_total {
        let ratio = layout.hierarchy_width / (layout.hierarchy_width + layout.inspector_width);
        layout.hierarchy_width =
            (max_side_total * ratio).clamp(HIERARCHY_WIDTH_MIN, HIERARCHY_WIDTH_MAX);
        layout.inspector_width = (max_side_total - layout.hierarchy_width)
            .clamp(INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX);
    }

    let max_bottom_total = display_h - VIEWPORT_MIN_HEIGHT;
    if layout.timeline_height + layout.debug_height > max_bottom_total {
        let ratio = layout.timeline_height / (layout.timeline_height + layout.debug_height);
        layout.timeline_height =
            (max_bottom_total * ratio).clamp(TIMELINE_HEIGHT_MIN, TIMELINE_HEIGHT_MAX);
        layout.debug_height =
            (max_bottom_total - layout.timeline_height).clamp(DEBUG_HEIGHT_MIN, DEBUG_HEIGHT_MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_prevents_viewport_too_small() {
        let mut layout = PanelLayout::default();
        layout.hierarchy_width = 800.0;
        layout.inspector_width = 800.0;
        panel_layout_clamp_to_display(&mut layout, 1920.0, 1080.0);

        let viewport_w = layout.viewport_width(1920.0);
        assert!(viewport_w >= VIEWPORT_MIN_WIDTH);
    }

    #[test]
    fn test_clamp_respects_min_max() {
        let mut layout = PanelLayout::default();
        layout.hierarchy_width = 10.0;
        layout.inspector_width = 10.0;
        layout.timeline_height = 10.0;
        layout.debug_height = 10.0;
        panel_layout_clamp_to_display(&mut layout, 1920.0, 1080.0);

        assert!(layout.hierarchy_width >= HIERARCHY_WIDTH_MIN);
        assert!(layout.inspector_width >= INSPECTOR_WIDTH_MIN);
        assert!(layout.timeline_height >= TIMELINE_HEIGHT_MIN);
        assert!(layout.debug_height >= DEBUG_HEIGHT_MIN);
    }

    #[test]
    fn test_clamp_vertical_constraints() {
        let mut layout = PanelLayout::default();
        layout.timeline_height = 600.0;
        layout.debug_height = 500.0;
        panel_layout_clamp_to_display(&mut layout, 1920.0, 1080.0);

        let main_h = layout.main_height(1080.0);
        assert!(main_h >= VIEWPORT_MIN_HEIGHT);
    }
}
