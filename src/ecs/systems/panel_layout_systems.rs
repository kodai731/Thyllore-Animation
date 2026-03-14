use crate::ecs::resource::PanelLayout;

pub fn panel_layout_clamp_to_display(layout: &mut PanelLayout, display_w: f32, display_h: f32) {
    layout.constrain_to_display(display_w, display_h);
}
