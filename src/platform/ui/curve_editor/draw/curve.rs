use crate::animation::editable::{
    curve_sample, segment_uses_bezier, EditableKeyframe, InterpolationType, PropertyCurve,
    TangentContinuity, TangentType, TangentWeightMode,
};
use crate::ecs::resource::CurveSelectedKeyframe;

use super::super::view::ViewTransform;

pub(in crate::platform::ui::curve_editor) fn draw_curve_with_keyframes(
    draw_list: &imgui::DrawListMut,
    curve: &PropertyCurve,
    color: [f32; 4],
    sample_count: usize,
    vt: &ViewTransform,
    _handle_times: Option<&[f32]>,
) {
    if curve.keyframes.is_empty() {
        return;
    }

    if curve.keyframes.len() == 1 {
        draw_keyframe_marker(draw_list, &curve.keyframes[0], color, vt);
        return;
    }

    let visible_start = vt.x_to_time(vt.curve_origin[0]);
    let visible_end = vt.x_to_time(vt.curve_origin[0] + vt.curve_width);
    if let (Some(first), Some(last)) = (curve.keyframes.first(), curve.keyframes.last()) {
        let pre_range = (visible_start, first.time.min(visible_end));
        let post_range = (last.time.max(visible_start), visible_end);
        draw_extrapolation(draw_list, curve, color, sample_count, vt, pre_range);
        draw_extrapolation(draw_list, curve, color, sample_count, vt, post_range);
    }

    for i in 0..curve.keyframes.len() - 1 {
        let k0 = &curve.keyframes[i];
        let k1 = &curve.keyframes[i + 1];

        let segment_samples = if k0.interpolation == InterpolationType::Stepped {
            let x0 = vt.time_to_x(k0.time);
            let y0 = vt.value_to_y(k0.value);
            let x1 = vt.time_to_x(k1.time);
            let y1 = vt.value_to_y(k1.value);
            draw_list
                .add_line([x0, y0], [x1, y0], color)
                .thickness(1.5)
                .build();
            draw_list
                .add_line([x1, y0], [x1, y1], color)
                .thickness(1.5)
                .build();
            continue;
        } else if segment_uses_bezier(k0, k1) {
            20
        } else {
            2
        };

        let mut prev_point: Option<[f32; 2]> = None;
        for s in 0..segment_samples {
            let frac = s as f32 / (segment_samples - 1) as f32;
            let time = k0.time + (k1.time - k0.time) * frac;
            if let Some(value) = curve_sample(curve, time) {
                let point = [vt.time_to_x(time), vt.value_to_y(value)];
                if let Some(prev) = prev_point {
                    draw_list
                        .add_line(prev, point, color)
                        .thickness(1.5)
                        .build();
                }
                prev_point = Some(point);
            }
        }
    }

    for kf in &curve.keyframes {
        draw_keyframe_marker(draw_list, kf, color, vt);
    }
}

const KEYFRAME_MARKER_RADIUS: f32 = 5.0;
const KEYFRAME_OUTLINE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.8];

#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyframeMarkerShape {
    HollowSquare,
    FilledSquare,
    Circle,
}

fn select_keyframe_marker_shape(kf: &EditableKeyframe) -> KeyframeMarkerShape {
    if kf.continuity == TangentContinuity::Broken {
        return KeyframeMarkerShape::HollowSquare;
    }
    match kf.tangent_type {
        TangentType::Manual => KeyframeMarkerShape::FilledSquare,
        TangentType::Spline
        | TangentType::Flat
        | TangentType::Linear
        | TangentType::Clamped
        | TangentType::Plateau => KeyframeMarkerShape::Circle,
    }
}

fn draw_keyframe_marker(
    draw_list: &imgui::DrawListMut,
    kf: &EditableKeyframe,
    color: [f32; 4],
    vt: &ViewTransform,
) {
    let center = [vt.time_to_x(kf.time), vt.value_to_y(kf.value)];
    let radius = KEYFRAME_MARKER_RADIUS;
    let min = [center[0] - radius, center[1] - radius];
    let max = [center[0] + radius, center[1] + radius];

    match select_keyframe_marker_shape(kf) {
        KeyframeMarkerShape::HollowSquare => {
            draw_list.add_rect(min, max, color).thickness(2.0).build();
        }
        KeyframeMarkerShape::FilledSquare => {
            draw_list.add_rect(min, max, color).filled(true).build();
            draw_list.add_rect(min, max, KEYFRAME_OUTLINE_COLOR).build();
        }
        KeyframeMarkerShape::Circle => {
            draw_list
                .add_circle(center, radius, color)
                .filled(true)
                .build();
            draw_list
                .add_circle(center, radius, KEYFRAME_OUTLINE_COLOR)
                .build();
        }
    }
}

const EXTRAPOLATION_DASH: f32 = 6.0;
const EXTRAPOLATION_GAP: f32 = 4.0;

fn draw_extrapolation(
    draw_list: &imgui::DrawListMut,
    curve: &PropertyCurve,
    color: [f32; 4],
    sample_count: usize,
    vt: &ViewTransform,
    (start_time, end_time): (f32, f32),
) {
    if end_time <= start_time || sample_count < 2 {
        return;
    }

    let points: Vec<[f32; 2]> = (0..sample_count)
        .filter_map(|s| {
            let time = start_time + (end_time - start_time) * s as f32 / (sample_count - 1) as f32;
            curve_sample(curve, time).map(|value| [vt.time_to_x(time), vt.value_to_y(value)])
        })
        .collect();

    for [from, to] in dashed_segments(&points, EXTRAPOLATION_DASH, EXTRAPOLATION_GAP) {
        draw_list.add_line(from, to, color).thickness(1.5).build();
    }
}

fn dashed_segments(points: &[[f32; 2]], dash: f32, gap: f32) -> Vec<[[f32; 2]; 2]> {
    let period = dash + gap;
    let mut segments = Vec::new();
    let mut phase = 0.0;

    for pair in points.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let length = (end[0] - start[0]).hypot(end[1] - start[1]);
        let point_at = |along: f32| {
            let t = along / length;
            [
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ]
        };

        let mut along = 0.0;
        while along < length {
            let drawing = phase < dash;
            let phase_end = if drawing { dash } else { period };
            let step = (phase_end - phase).min(length - along);
            if drawing {
                segments.push([point_at(along), point_at(along + step)]);
            }
            along += step;
            phase += step;
            if phase >= period {
                phase -= period;
            }
        }
    }
    segments
}

pub(in crate::platform::ui::curve_editor) fn draw_selected_keyframes_highlight(
    draw_list: &imgui::DrawListMut,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
    vt: &ViewTransform,
) {
    for selected in selected_keyframes {
        for (curve, _, _) in curves_to_draw {
            if curve.property_type == selected.property_type {
                if let Some(kf) = curve.get_keyframe(selected.keyframe_id) {
                    let curve_vt = vt.for_curve(curve);
                    let x = curve_vt.time_to_x(kf.time);
                    let y = curve_vt.value_to_y(kf.value);

                    draw_list
                        .add_circle([x, y], 8.0, [1.0, 1.0, 0.0, 1.0])
                        .thickness(2.0)
                        .build();
                }
                break;
            }
        }
    }
}

pub(in crate::platform::ui::curve_editor) fn draw_tangent_handles(
    draw_list: &imgui::DrawListMut,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
    vt: &ViewTransform,
) {
    for selected in selected_keyframes {
        for (curve, color, _) in curves_to_draw {
            if curve.property_type != selected.property_type {
                continue;
            }

            let kf = match curve.get_keyframe(selected.keyframe_id) {
                Some(kf) => kf,
                None => break,
            };

            if kf.interpolation != InterpolationType::Bezier {
                break;
            }

            let curve_vt = vt.for_curve(curve);
            let kf_x = curve_vt.time_to_x(kf.time);
            let kf_y = curve_vt.value_to_y(kf.value);
            let handle_color = [color[0], color[1], color[2], 0.9];
            let handle_size = 4.0;
            let is_weighted = kf.weight_mode == TangentWeightMode::Weighted;

            let in_x = curve_vt.time_to_x(kf.time + kf.in_tangent.time_offset);
            let in_y = curve_vt.value_to_y(kf.value + kf.in_tangent.value_offset);
            draw_list
                .add_line([kf_x, kf_y], [in_x, in_y], handle_color)
                .thickness(1.0)
                .build();
            if is_weighted {
                draw_list
                    .add_circle([in_x, in_y], handle_size, handle_color)
                    .filled(true)
                    .build();
            } else {
                draw_list
                    .add_rect(
                        [in_x - handle_size, in_y - handle_size],
                        [in_x + handle_size, in_y + handle_size],
                        handle_color,
                    )
                    .filled(true)
                    .build();
            }

            let out_x = curve_vt.time_to_x(kf.time + kf.out_tangent.time_offset);
            let out_y = curve_vt.value_to_y(kf.value + kf.out_tangent.value_offset);
            draw_list
                .add_line([kf_x, kf_y], [out_x, out_y], handle_color)
                .thickness(1.0)
                .build();
            if is_weighted {
                draw_list
                    .add_circle([out_x, out_y], handle_size, handle_color)
                    .filled(true)
                    .build();
            } else {
                draw_list
                    .add_rect(
                        [out_x - handle_size, out_y - handle_size],
                        [out_x + handle_size, out_y + handle_size],
                        handle_color,
                    )
                    .filled(true)
                    .build();
            }

            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashes_restart_after_each_gap() {
        let segments = dashed_segments(&[[0.0, 0.0], [20.0, 0.0]], 6.0, 4.0);

        assert_eq!(
            segments,
            vec![[[0.0, 0.0], [6.0, 0.0]], [[10.0, 0.0], [16.0, 0.0]]]
        );
    }

    #[test]
    fn a_dash_continues_across_a_corner() {
        let segments = dashed_segments(&[[0.0, 0.0], [4.0, 0.0], [4.0, 8.0]], 6.0, 4.0);

        assert_eq!(
            segments,
            vec![
                [[0.0, 0.0], [4.0, 0.0]],
                [[4.0, 0.0], [4.0, 2.0]],
                [[4.0, 6.0], [4.0, 8.0]],
            ]
        );
    }

    #[test]
    fn broken_key_is_a_hollow_square_whatever_its_tangent_type() {
        let mut kf = EditableKeyframe::new(0, 0.0, 0.0);
        kf.continuity = TangentContinuity::Broken;
        kf.tangent_type = TangentType::Clamped;

        assert_eq!(
            select_keyframe_marker_shape(&kf),
            KeyframeMarkerShape::HollowSquare
        );
    }

    #[test]
    fn unified_manual_key_is_a_filled_square_and_auto_key_a_circle() {
        let mut kf = EditableKeyframe::new(0, 0.0, 0.0);
        assert_eq!(
            select_keyframe_marker_shape(&kf),
            KeyframeMarkerShape::FilledSquare
        );

        kf.tangent_type = TangentType::Spline;
        assert_eq!(
            select_keyframe_marker_shape(&kf),
            KeyframeMarkerShape::Circle
        );
    }
}
