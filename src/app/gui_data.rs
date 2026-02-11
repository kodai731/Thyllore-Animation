use crate::app::data::LightMoveTarget;
use cgmath::{InnerSpace, Vector2};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct GUIData {
    pub is_left_clicked: bool,
    pub is_right_clicked: bool,
    pub is_wheel_clicked: bool,
    pub monitor_value: f32,
    pub mouse_pos: [f32; 2],
    pub mouse_wheel: f32,
    pub mouse_diff: [f32; 2],
    pub file_path: String,
    pub file_changed: bool,
    pub selected_model_path: String,
    pub load_status: String,
    pub take_screenshot: bool,
    pub imgui_wants_mouse: bool,
    pub show_click_debug: bool,
    pub billboard_click_rect: Option<[f32; 4]>,
    pub debug_billboard_depth: bool,
    pub show_light_ray_to_model: bool,
    pub is_ctrl_pressed: bool,
    pub is_shift_pressed: bool,
    pub move_light_to: LightMoveTarget,
    pub clicked_mouse_pos: Option<[f32; 2]>,
    pub viewport_resize_pending: Option<(u32, u32)>,
    pub viewport_position: [f32; 2],
    pub viewport_size: [f32; 2],
    pub viewport_hovered: bool,
    pub viewport_focused: bool,
}

impl Default for GUIData {
    fn default() -> Self {
        Self {
            is_left_clicked: false,
            is_right_clicked: false,
            is_wheel_clicked: false,
            monitor_value: 0.0,
            mouse_pos: [0.0, 0.0],
            mouse_wheel: 0.0,
            mouse_diff: [0.0, 0.0],
            file_path: String::default(),
            file_changed: false,
            selected_model_path: String::default(),
            load_status: String::from("No model loaded"),
            take_screenshot: false,
            imgui_wants_mouse: false,
            show_click_debug: false,
            billboard_click_rect: None,
            debug_billboard_depth: false,
            show_light_ray_to_model: false,
            is_ctrl_pressed: false,
            is_shift_pressed: false,
            move_light_to: LightMoveTarget::None,
            clicked_mouse_pos: None,
            viewport_resize_pending: None,
            viewport_position: [0.0, 0.0],
            viewport_size: [0.0, 0.0],
            viewport_hovered: false,
            viewport_focused: false,
        }
    }
}

impl GUIData {
    pub fn update(&mut self) {
        self.mouse_diff = [0.0, 0.0];

        let allow_input = !self.imgui_wants_mouse || self.viewport_hovered;
        if !allow_input {
            self.clicked_mouse_pos = None;
            return;
        }

        let mouse_pos = Vector2::new(self.mouse_pos[0], self.mouse_pos[1]);
        let is_dragging = self.is_right_clicked || self.is_wheel_clicked;

        if is_dragging {
            if self.clicked_mouse_pos.is_none() {
                self.clicked_mouse_pos = Some([mouse_pos.x, mouse_pos.y]);
            }

            let clicked = self
                .clicked_mouse_pos
                .map(|p| Vector2::new(p[0], p[1]))
                .unwrap_or(mouse_pos);

            let diff = mouse_pos - clicked;
            if diff.magnitude() > 0.001 {
                self.mouse_diff = [diff.x, diff.y];
                self.clicked_mouse_pos = Some([mouse_pos.x, mouse_pos.y]);
            }
        } else {
            self.clicked_mouse_pos = None;
        }
    }
}
