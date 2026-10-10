use crate::animation::editable::PropertyCurve;
use crate::ecs::resource::{
    CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe, FrameRequest,
};
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

const FRAME_PADDING_RATIO: f32 = 0.1;
const MIN_FRAMED_TIME_RATIO: f32 = 0.05;
const MIN_FRAMED_VALUE_SPAN: f32 = 0.1;

pub(super) struct FrameBounds {
    pub(super) time_min: f32,
    pub(super) time_max: f32,
    pub(super) value_min: f32,
    pub(super) value_max: f32,
}

impl FrameBounds {
    fn from_point(time: f32, value: f32) -> Self {
        Self {
            time_min: time,
            time_max: time,
            value_min: value,
            value_max: value,
        }
    }

    fn include(self, time: f32, value: f32) -> Self {
        Self {
            time_min: self.time_min.min(time),
            time_max: self.time_max.max(time),
            value_min: self.value_min.min(value),
            value_max: self.value_max.max(value),
        }
    }
}

pub(super) struct FrameView {
    pub(super) zoom_x: f32,
    pub(super) zoom_y: f32,
    pub(super) time_offset: f32,
    pub(super) value_offset: f32,
}

pub(super) fn compute_frame_view(
    bounds: FrameBounds,
    view_duration: f32,
    view_val_range: f32,
) -> FrameView {
    let content_ratio = 1.0 - 2.0 * FRAME_PADDING_RATIO;
    let time_span = (bounds.time_max - bounds.time_min).max(view_duration * MIN_FRAMED_TIME_RATIO);
    let value_span = (bounds.value_max - bounds.value_min).max(MIN_FRAMED_VALUE_SPAN);

    let visible_time_span = time_span / content_ratio;
    let visible_value_span = value_span / content_ratio;
    let time_center = (bounds.time_min + bounds.time_max) * 0.5;
    let value_center = (bounds.value_min + bounds.value_max) * 0.5;

    FrameView {
        zoom_x: view_duration / visible_time_span,
        zoom_y: view_val_range / visible_value_span,
        time_offset: time_center - visible_time_span * 0.5,
        value_offset: value_center - visible_value_span * 0.5,
    }
}

pub(super) fn center_on_time(time: f32, view_duration: f32, zoom_x: f32) -> f32 {
    time - view_duration / zoom_x * 0.5
}

fn merge_into_bounds(points: impl Iterator<Item = (f32, f32)>) -> Option<FrameBounds> {
    points.fold(None, |bounds, (time, value)| match bounds {
        Some(bounds) => Some(bounds.include(time, value)),
        None => Some(FrameBounds::from_point(time, value)),
    })
}

fn collect_all_keyframe_bounds(curves: &[(&PropertyCurve, [f32; 4], &str)]) -> Option<FrameBounds> {
    merge_into_bounds(
        curves
            .iter()
            .flat_map(|(curve, _, _)| curve.keyframes.iter())
            .map(|keyframe| (keyframe.time, keyframe.value)),
    )
}

fn collect_selected_keyframe_bounds(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
) -> Option<FrameBounds> {
    if selected_keyframes.is_empty() {
        return collect_all_keyframe_bounds(curves);
    }

    merge_into_bounds(selected_keyframes.iter().filter_map(|selected| {
        curves
            .iter()
            .find(|(curve, _, _)| curve.property_type == selected.property_type)
            .and_then(|(curve, _, _)| curve.get_keyframe(selected.keyframe_id))
            .map(|keyframe| (keyframe.time, keyframe.value))
    }))
}

pub(super) fn apply_frame_request(
    editor_state: &mut CurveEditorState,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    current_time: f32,
) {
    let Some(request) = editor_state.frame_request.take() else {
        return;
    };

    let bounds = match request {
        FrameRequest::Selected => {
            collect_selected_keyframe_bounds(curves, &editor_state.selected_keyframes)
        }
        FrameRequest::All => collect_all_keyframe_bounds(curves),
        FrameRequest::Playhead => {
            editor_state.view_time_offset = center_on_time(
                current_time,
                editor_state.view_duration,
                editor_state.zoom_x,
            );
            return;
        }
    };
    let Some(bounds) = bounds else {
        return;
    };

    let frame_view = compute_frame_view(
        bounds,
        editor_state.view_duration,
        editor_state.view_val_range,
    );
    editor_state.zoom_x = frame_view.zoom_x;
    editor_state.zoom_y = frame_view.zoom_y;
    editor_state.view_time_offset = frame_view.time_offset;
    editor_state.view_value_offset = frame_view.value_offset;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{curve_add_keyframe, PropertyType};

    const TOLERANCE: f32 = 1e-4;

    fn build_curve(keyframes: &[(f32, f32)]) -> PropertyCurve {
        let mut curve = PropertyCurve::new(0, PropertyType::TranslationX);
        for &(time, value) in keyframes {
            curve_add_keyframe(&mut curve, time, value);
        }
        curve
    }

    fn build_editor_state(request: FrameRequest) -> CurveEditorState {
        CurveEditorState {
            view_duration: 2.0,
            view_val_range: 2.0,
            frame_request: Some(request),
            ..Default::default()
        }
    }

    fn build_unit_view(editor_state: &CurveEditorState) -> ViewTransform {
        ViewTransform {
            curve_origin: [0.0, 0.0],
            curve_width: 1.0,
            curve_height: 1.0,
            duration: editor_state.view_duration,
            val_range: editor_state.view_val_range,
            zoom_x: editor_state.zoom_x,
            zoom_y: editor_state.zoom_y,
            view_time_offset: editor_state.view_time_offset,
            view_value_offset: editor_state.view_value_offset,
        }
    }

    #[test]
    fn framing_two_keys_places_them_at_the_padding_edges() {
        let curve = build_curve(&[(1.0, 0.0), (2.0, 10.0)]);
        let curves = [(&curve, [1.0; 4], "x")];
        let mut editor_state = build_editor_state(FrameRequest::All);

        apply_frame_request(&mut editor_state, &curves, 0.0);
        let view = build_unit_view(&editor_state);

        assert!((view.time_to_x(1.0) - FRAME_PADDING_RATIO).abs() < TOLERANCE);
        assert!((view.time_to_x(2.0) - (1.0 - FRAME_PADDING_RATIO)).abs() < TOLERANCE);
        assert!((view.value_to_y(10.0) - FRAME_PADDING_RATIO).abs() < TOLERANCE);
        assert!((view.value_to_y(0.0) - (1.0 - FRAME_PADDING_RATIO)).abs() < TOLERANCE);
        assert!(editor_state.frame_request.is_none());
    }

    #[test]
    fn framing_a_single_key_keeps_zoom_finite() {
        let curve = build_curve(&[(1.0, 5.0)]);
        let curves = [(&curve, [1.0; 4], "x")];
        let mut editor_state = build_editor_state(FrameRequest::Selected);

        apply_frame_request(&mut editor_state, &curves, 0.0);
        let view = build_unit_view(&editor_state);

        assert!(editor_state.zoom_x.is_finite() && editor_state.zoom_x > 0.0);
        assert!(editor_state.zoom_y.is_finite() && editor_state.zoom_y > 0.0);
        assert!((view.time_to_x(1.0) - 0.5).abs() < TOLERANCE);
        assert!((view.value_to_y(5.0) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn framing_the_playhead_centers_current_time_and_keeps_zoom() {
        let curve = build_curve(&[(1.0, 0.0), (2.0, 10.0)]);
        let curves = [(&curve, [1.0; 4], "x")];
        let mut editor_state = build_editor_state(FrameRequest::Playhead);
        editor_state.zoom_x = 3.0;

        apply_frame_request(&mut editor_state, &curves, 1.7);
        let view = build_unit_view(&editor_state);

        assert_eq!(editor_state.zoom_x, 3.0);
        assert!((view.time_to_x(1.7) - 0.5).abs() < TOLERANCE);
    }
}
