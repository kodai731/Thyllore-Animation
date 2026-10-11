use crate::animation::editable::{
    sample_bezier, segment_uses_bezier, BezierHandle, EditableKeyframe, InterpolationType,
    PropertyCurve,
};
use crate::ecs::resource::{AxisLock, CurveSelectedKeyframe, DraggingTangent, TangentHandleType};

use super::super::interaction::{dragged_key_delta, dragged_key_position, TimeSnap};
use super::super::view::ViewTransform;

pub(in crate::platform::ui::curve_editor) fn draw_keyframe_drag_preview(
    draw_list: &imgui::DrawListMut,
    mouse_pos: [f32; 2],
    drag_start: [f32; 2],
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
    axis_lock: Option<AxisLock>,
    time_snap: TimeSnap,
) {
    for sel in selected_keyframes {
        let curve = match curves_to_draw
            .iter()
            .find(|(c, _, _)| c.property_type == sel.property_type)
        {
            Some((curve, _, _)) => *curve,
            None => continue,
        };
        let curve_vt = vt.for_curve(curve);

        let delta = dragged_key_delta(&curve_vt, drag_start, mouse_pos);

        let [preview_time, preview_value] = dragged_key_position(
            [sel.original_time, sel.original_value],
            delta,
            axis_lock,
            time_snap,
        );
        let preview_x = curve_vt.time_to_x(preview_time).clamp(
            curve_vt.curve_origin[0],
            curve_vt.curve_origin[0] + curve_vt.curve_width,
        );
        let preview_y = curve_vt.value_to_y(preview_value).clamp(
            curve_vt.curve_origin[1],
            curve_vt.curve_origin[1] + curve_vt.curve_height,
        );
        let preview_pos = [preview_x, preview_y];

        draw_drag_neighbor_lines(draw_list, preview_pos, &curve_vt, curves_to_draw, sel);

        draw_list
            .add_circle(preview_pos, 7.0, [1.0, 1.0, 0.0, 1.0])
            .filled(true)
            .build();

        draw_list
            .add_circle(preview_pos, 7.0, [1.0, 1.0, 1.0, 1.0])
            .thickness(2.0)
            .build();

        draw_list.add_text(
            [preview_x + 10.0, preview_y - 10.0],
            [1.0, 1.0, 1.0, 1.0],
            &format!("t={:.2}s v={:.3}", preview_time, preview_value),
        );
    }
}

pub(in crate::platform::ui::curve_editor) fn draw_drag_neighbor_lines(
    draw_list: &imgui::DrawListMut,
    preview_pos: [f32; 2],
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    selected: &CurveSelectedKeyframe,
) {
    for (curve, color, _) in curves_to_draw {
        if curve.property_type != selected.property_type {
            continue;
        }

        let kf_index = match curve
            .keyframes
            .iter()
            .position(|kf| kf.id == selected.keyframe_id)
        {
            Some(idx) => idx,
            None => break,
        };

        let line_color = [color[0], color[1], color[2], 0.6];

        if kf_index > 0 {
            let prev = &curve.keyframes[kf_index - 1];
            let prev_pos = [vt.time_to_x(prev.time), vt.value_to_y(prev.value)];
            draw_list
                .add_line(prev_pos, preview_pos, line_color)
                .thickness(1.5)
                .build();
        }

        if kf_index + 1 < curve.keyframes.len() {
            let next = &curve.keyframes[kf_index + 1];
            let next_pos = [vt.time_to_x(next.time), vt.value_to_y(next.value)];
            draw_list
                .add_line(preview_pos, next_pos, line_color)
                .thickness(1.5)
                .build();
        }

        break;
    }
}

pub(in crate::platform::ui::curve_editor) fn draw_tangent_drag_curve_preview(
    draw_list: &imgui::DrawListMut,
    dragging: &DraggingTangent,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) {
    let (curve, color) = match curves_to_draw
        .iter()
        .find(|(c, _, _)| c.property_type == dragging.property_type)
    {
        Some((c, col, _)) => (*c, *col),
        None => return,
    };

    let kf_idx = match curve
        .keyframes
        .iter()
        .position(|k| k.id == dragging.keyframe_id)
    {
        Some(idx) => idx,
        None => return,
    };

    let curve_vt = vt.for_curve(curve);

    let mouse_time = curve_vt.x_to_time(mouse_pos[0]);
    let mouse_value = curve_vt.y_to_value(mouse_pos[1]);
    let kf = &curve.keyframes[kf_idx];
    let new_handle = BezierHandle::new(mouse_time - kf.time, mouse_value - kf.value);

    let preview_in = match dragging.handle_type {
        TangentHandleType::In => new_handle.clone(),
        TangentHandleType::Out => kf.in_tangent.clone(),
    };
    let preview_out = match dragging.handle_type {
        TangentHandleType::In => kf.out_tangent.clone(),
        TangentHandleType::Out => new_handle.clone(),
    };

    let preview_color = [
        (color[0] + 1.0) * 0.5,
        (color[1] + 1.0) * 0.5,
        (color[2] + 1.0) * 0.5,
        0.9,
    ];

    draw_preview_segment_before(
        draw_list,
        curve,
        kf_idx,
        &preview_in,
        preview_color,
        &curve_vt,
    );
    draw_preview_segment_after(
        draw_list,
        curve,
        kf_idx,
        &preview_out,
        preview_color,
        &curve_vt,
    );

    let kf_x = curve_vt.time_to_x(kf.time);
    let kf_y = curve_vt.value_to_y(kf.value);
    let handle_x = mouse_pos[0];
    let handle_y = mouse_pos[1];

    draw_list
        .add_line([kf_x, kf_y], [handle_x, handle_y], [1.0, 1.0, 0.0, 0.8])
        .thickness(1.0)
        .build();
    draw_list
        .add_rect(
            [handle_x - 5.0, handle_y - 5.0],
            [handle_x + 5.0, handle_y + 5.0],
            [1.0, 1.0, 0.0, 0.8],
        )
        .filled(true)
        .build();
}

pub(in crate::platform::ui::curve_editor) fn draw_preview_segment_before(
    draw_list: &imgui::DrawListMut,
    curve: &PropertyCurve,
    kf_idx: usize,
    preview_in: &BezierHandle,
    color: [f32; 4],
    vt: &ViewTransform,
) {
    if kf_idx == 0 {
        return;
    }

    let k0 = &curve.keyframes[kf_idx - 1];
    let k1 = &curve.keyframes[kf_idx];

    if k0.interpolation == InterpolationType::Stepped {
        return;
    }

    if !segment_uses_bezier(k0, k1) {
        return;
    }

    draw_preview_bezier_segment(draw_list, k0, k1, &k0.out_tangent, preview_in, color, vt);
}

pub(in crate::platform::ui::curve_editor) fn draw_preview_segment_after(
    draw_list: &imgui::DrawListMut,
    curve: &PropertyCurve,
    kf_idx: usize,
    preview_out: &BezierHandle,
    color: [f32; 4],
    vt: &ViewTransform,
) {
    if kf_idx + 1 >= curve.keyframes.len() {
        return;
    }

    let k0 = &curve.keyframes[kf_idx];
    let k1 = &curve.keyframes[kf_idx + 1];

    if k0.interpolation == InterpolationType::Stepped {
        return;
    }

    if !segment_uses_bezier(k0, k1) {
        return;
    }

    draw_preview_bezier_segment(draw_list, k0, k1, preview_out, &k1.in_tangent, color, vt);
}

pub(in crate::platform::ui::curve_editor) fn draw_preview_bezier_segment(
    draw_list: &imgui::DrawListMut,
    k0: &EditableKeyframe,
    k1: &EditableKeyframe,
    out_handle: &BezierHandle,
    in_handle: &BezierHandle,
    color: [f32; 4],
    vt: &ViewTransform,
) {
    let samples = 20;
    let mut prev_point: Option<[f32; 2]> = None;

    for s in 0..samples {
        let frac = s as f32 / (samples - 1) as f32;
        let time = k0.time + (k1.time - k0.time) * frac;

        let dt = k1.time - k0.time;
        let dv = k1.value - k0.value;

        let effective_out = if k0.interpolation == InterpolationType::Bezier {
            out_handle.clone()
        } else {
            BezierHandle::new(dt / 3.0, dv / 3.0)
        };
        let effective_in = if k1.interpolation == InterpolationType::Bezier {
            in_handle.clone()
        } else {
            BezierHandle::new(-dt / 3.0, -dv / 3.0)
        };

        let value = sample_bezier(
            k0.time,
            k0.value,
            &effective_out,
            k1.time,
            k1.value,
            &effective_in,
            time,
        );

        let point = [vt.time_to_x(time), vt.value_to_y(value)];
        if let Some(prev) = prev_point {
            draw_list
                .add_line(prev, point, color)
                .thickness(2.5)
                .build();
        }
        prev_point = Some(point);
    }
}
