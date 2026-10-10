use crate::animation::editable::PropertyCurve;
use crate::ecs::resource::{CurveEditorState, CurveInteractionMode};
use crate::platform::ui::pointer::UiPointer;

const PAN_SPEED: f32 = 30.0;

pub(super) struct ViewTransform {
    pub(super) curve_origin: [f32; 2],
    pub(super) curve_width: f32,
    pub(super) curve_height: f32,
    pub(super) duration: f32,
    pub(super) val_range: f32,
    pub(super) zoom_x: f32,
    pub(super) zoom_y: f32,
    pub(super) view_time_offset: f32,
    pub(super) view_value_offset: f32,
}

impl ViewTransform {
    pub(super) fn time_to_x(&self, time: f32) -> f32 {
        self.curve_origin[0]
            + (time - self.view_time_offset) / self.duration.max(0.001)
                * self.zoom_x
                * self.curve_width
    }

    pub(super) fn value_to_y(&self, value: f32) -> f32 {
        self.curve_origin[1] + self.curve_height
            - (value - self.view_value_offset) / self.val_range.max(0.001)
                * self.zoom_y
                * self.curve_height
    }

    pub(super) fn x_to_time(&self, x: f32) -> f32 {
        (x - self.curve_origin[0]) / (self.zoom_x * self.curve_width).max(0.001)
            * self.duration.max(0.001)
            + self.view_time_offset
    }

    pub(super) fn y_to_value(&self, y: f32) -> f32 {
        (self.curve_origin[1] + self.curve_height - y)
            / (self.zoom_y * self.curve_height).max(0.001)
            * self.val_range.max(0.001)
            + self.view_value_offset
    }
}

pub(super) fn initialize_view_range(
    editor_state: &mut CurveEditorState,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    clip_duration: f32,
) {
    let (global_min, global_max) = calculate_global_value_range(curves_to_draw);
    let display_duration = clip_duration;

    if !editor_state.view_initialized {
        editor_state.view_value_offset = global_min;
        editor_state.view_val_range = global_max - global_min;
        editor_state.view_duration = display_duration;
        editor_state.view_time_offset = 0.0;
        editor_state.zoom_x = 1.0;
        editor_state.zoom_y = 1.0;
        editor_state.view_initialized = true;
    } else {
        editor_state.view_duration = editor_state.view_duration.max(display_duration);
    }
}

pub(super) fn handle_panning(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    vt: &ViewTransform,
) {
    let CurveInteractionMode::Panning {
        start_mouse_pos,
        start_offset,
    } = editor_state.interaction
    else {
        return;
    };

    let dx = mouse_pos[0] - start_mouse_pos[0];
    let dy = mouse_pos[1] - start_mouse_pos[1];

    let time_per_pixel = vt.duration.max(0.001) / (vt.zoom_x * vt.curve_width).max(0.001);
    let value_per_pixel = vt.val_range.max(0.001) / (vt.zoom_y * vt.curve_height).max(0.001);

    editor_state.view_time_offset = start_offset[0] - dx * time_per_pixel;
    editor_state.view_value_offset = start_offset[1] + dy * value_per_pixel;
}

pub(super) fn handle_wheel_input(
    ui: &imgui::Ui,
    editor_state: &mut CurveEditorState,
    pointer: &UiPointer,
    vt: &ViewTransform,
) {
    let mouse_pos = pointer.pos;
    let wheel = pointer.wheel;
    if wheel == 0.0 {
        return;
    }

    let ctrl = ui.io().key_ctrl;

    if ctrl {
        zoom_at_mouse(editor_state, mouse_pos, wheel, vt);
    } else {
        let shift = ui.io().key_shift;
        pan_with_wheel(editor_state, wheel, shift, vt);
    }
}

pub(super) fn zoom_at_mouse(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    wheel: f32,
    vt: &ViewTransform,
) {
    let mouse_time = vt.x_to_time(mouse_pos[0]);
    let mouse_value = vt.y_to_value(mouse_pos[1]);

    let factor = if wheel > 0.0 { 1.15 } else { 1.0 / 1.15 };
    let new_zoom_x = (editor_state.zoom_x * factor).clamp(0.1, 10.0);
    let new_zoom_y = (editor_state.zoom_y * factor).clamp(0.1, 10.0);

    editor_state.view_time_offset = mouse_time
        - (mouse_pos[0] - vt.curve_origin[0]) / (new_zoom_x * vt.curve_width).max(0.001)
            * vt.duration.max(0.001);

    editor_state.view_value_offset = mouse_value
        - (vt.curve_origin[1] + vt.curve_height - mouse_pos[1])
            / (new_zoom_y * vt.curve_height).max(0.001)
            * vt.val_range.max(0.001);

    editor_state.zoom_x = new_zoom_x;
    editor_state.zoom_y = new_zoom_y;
}

pub(super) fn pan_with_wheel(
    editor_state: &mut CurveEditorState,
    wheel: f32,
    shift: bool,
    vt: &ViewTransform,
) {
    if shift {
        let time_per_pixel = vt.duration.max(0.001) / (vt.zoom_x * vt.curve_width).max(0.001);
        editor_state.view_time_offset -= wheel * PAN_SPEED * time_per_pixel;
    } else {
        let value_per_pixel = vt.val_range.max(0.001) / (vt.zoom_y * vt.curve_height).max(0.001);
        editor_state.view_value_offset += wheel * PAN_SPEED * value_per_pixel;
    }
}

pub(super) fn calculate_sample_count(width: f32) -> usize {
    let base_samples = 60;
    let samples_per_100px = 15;
    let additional = ((width / 100.0) as usize) * samples_per_100px;
    (base_samples + additional).min(200)
}

pub(super) fn calculate_y_tick_count(height: f32) -> usize {
    ((height / 40.0) as usize).max(2).min(15)
}

pub(super) fn compute_nice_step(raw_step: f32) -> f32 {
    if raw_step <= 0.0 {
        return 1.0;
    }
    let magnitude = 10.0f32.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;

    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice * magnitude
}

pub(super) fn format_value_label(value: f32) -> String {
    let abs = value.abs();
    if abs >= 100.0 {
        format!("{:.0}", value)
    } else if abs >= 1.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.2}", value)
    }
}

pub(super) fn calculate_global_value_range(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
) -> (f32, f32) {
    let mut min_val = f32::MAX;
    let mut max_val = f32::MIN;

    for (curve, _, _) in curves {
        for kf in &curve.keyframes {
            min_val = min_val.min(kf.value);
            max_val = max_val.max(kf.value);
        }
    }

    if min_val == f32::MAX {
        min_val = -1.0;
        max_val = 1.0;
    } else if (max_val - min_val).abs() < 0.001 {
        min_val -= 0.5;
        max_val += 0.5;
    }

    let padding = (max_val - min_val) * 0.1;
    (min_val - padding, max_val + padding)
}
