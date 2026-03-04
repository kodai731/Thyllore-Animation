use crate::ecs::resource::PanelLayout;

#[derive(Clone, Debug)]
pub struct LayoutSnapshot {
    pub hierarchy_width: f32,
    pub inspector_width: f32,
    pub timeline_height: f32,
    pub debug_height: f32,
    pub main_height: f32,
    pub viewport_width: f32,
    pub viewport_x: f32,
    pub inspector_x: f32,
    pub timeline_y: f32,
    pub debug_y: f32,
    pub hierarchy_height: f32,
    pub clip_browser_y: f32,
    pub clip_browser_height: f32,
    pub display_size: [f32; 2],
}

impl LayoutSnapshot {
    pub fn from_layout(layout: &PanelLayout, display_size: [f32; 2]) -> Self {
        let main_height = layout.main_height(display_size[1]);
        let viewport_width = layout.viewport_width(display_size[0]);
        let hierarchy_height = (main_height * 0.6).max(100.0);
        let clip_browser_height = (main_height - hierarchy_height).max(80.0);

        Self {
            hierarchy_width: layout.hierarchy_width,
            inspector_width: layout.inspector_width,
            timeline_height: layout.timeline_height,
            debug_height: layout.debug_height,
            main_height,
            viewport_width,
            viewport_x: layout.hierarchy_width,
            inspector_x: display_size[0] - layout.inspector_width,
            timeline_y: main_height,
            debug_y: display_size[1] - layout.debug_height,
            hierarchy_height,
            clip_browser_y: hierarchy_height,
            clip_browser_height,
            display_size,
        }
    }
}
