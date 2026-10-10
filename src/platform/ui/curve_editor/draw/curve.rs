use crate::animation::editable::{
    curve_sample, segment_uses_bezier, InterpolationType, PropertyCurve, TangentWeightMode,
};
use crate::ecs::resource::CurveSelectedKeyframe;

use super::super::view::ViewTransform;

pub(in crate::platform::ui::curve_editor) fn draw_curve_with_keyframes(
    draw_list: &imgui::DrawListMut,
    curve: &PropertyCurve,
    color: [f32; 4],
    _sample_count: usize,
    vt: &ViewTransform,
    _handle_times: Option<&[f32]>,
) {
    if curve.keyframes.is_empty() {
        return;
    }

    if curve.keyframes.len() == 1 {
        let kf = &curve.keyframes[0];
        let x = vt.time_to_x(kf.time);
        let y = vt.value_to_y(kf.value);
        draw_list
            .add_circle([x, y], 5.0, color)
            .filled(true)
            .build();
        draw_list
            .add_circle([x, y], 5.0, [1.0, 1.0, 1.0, 0.8])
            .build();
        return;
    }

    if let Some(first) = curve.keyframes.first() {
        let first_x = vt.time_to_x(first.time);
        let start_x = vt.time_to_x(0.0);
        if start_x < first_x {
            let y = vt.value_to_y(first.value);
            draw_list
                .add_line([start_x, y], [first_x, y], color)
                .thickness(1.5)
                .build();
        }
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

    if let Some(last) = curve.keyframes.last() {
        let last_x = vt.time_to_x(last.time);
        let end_x = vt.time_to_x(vt.duration);
        if end_x > last_x {
            let y = vt.value_to_y(last.value);
            draw_list
                .add_line([last_x, y], [end_x, y], color)
                .thickness(1.5)
                .build();
        }
    }

    for kf in &curve.keyframes {
        let x = vt.time_to_x(kf.time);
        let y = vt.value_to_y(kf.value);

        draw_list
            .add_circle([x, y], 5.0, color)
            .filled(true)
            .build();

        draw_list
            .add_circle([x, y], 5.0, [1.0, 1.0, 1.0, 0.8])
            .build();
    }
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
