use cgmath::{InnerSpace, Vector2};

#[derive(Clone, Debug, Default)]
pub struct MouseInput {
    pub position: [f32; 2],
    pub delta: [f32; 2],
    pub wheel: f32,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub middle_pressed: bool,
    prev_drag_pos: Option<[f32; 2]>,
}

impl MouseInput {
    pub fn compute_drag_delta(&mut self, imgui_wants_mouse: bool, viewport_hovered: bool) {
        self.delta = [0.0, 0.0];

        let allow_input = !imgui_wants_mouse || viewport_hovered;
        if !allow_input {
            self.prev_drag_pos = None;
            return;
        }

        let mouse_pos = Vector2::new(self.position[0], self.position[1]);
        let is_dragging = self.right_pressed || self.middle_pressed;

        if is_dragging {
            if self.prev_drag_pos.is_none() {
                self.prev_drag_pos = Some([mouse_pos.x, mouse_pos.y]);
            }

            let clicked = self
                .prev_drag_pos
                .map(|p| Vector2::new(p[0], p[1]))
                .unwrap_or(mouse_pos);

            let diff = mouse_pos - clicked;
            if diff.magnitude() > 0.001 {
                self.delta = [diff.x, diff.y];
                self.prev_drag_pos = Some([mouse_pos.x, mouse_pos.y]);
            }
        } else {
            self.prev_drag_pos = None;
        }
    }

    pub fn end_frame(&mut self) {
        self.wheel = 0.0;
    }
}
