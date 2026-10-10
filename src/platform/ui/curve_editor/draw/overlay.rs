use std::collections::HashSet;

use crate::animation::editable::PropertyType;
use crate::animation::BoneId;
use crate::ecs::resource::CurveEditorBuffer;

use super::super::view::ViewTransform;
use super::super::{SuggestionOverlay, ALL_PROPERTY_TYPES};

pub(in crate::platform::ui::curve_editor) fn draw_buffer_curve_overlay(
    draw_list: &imgui::DrawListMut,
    buffer: &CurveEditorBuffer,
    bone_id: BoneId,
    visible_curves: &HashSet<PropertyType>,
    vt: &ViewTransform,
) {
    if buffer.is_empty() {
        return;
    }

    for (prop_type, color, _name) in ALL_PROPERTY_TYPES {
        if !visible_curves.contains(prop_type) {
            continue;
        }

        let snapshot = match buffer.get_snapshot(bone_id, *prop_type) {
            Some(s) => s,
            None => continue,
        };

        if snapshot.len() < 2 {
            continue;
        }

        let ghost_color = [color[0], color[1], color[2], 0.35];

        for i in 0..snapshot.len() - 1 {
            let (t0, v0) = snapshot[i];
            let (t1, v1) = snapshot[i + 1];

            let x0 = vt.time_to_x(t0);
            let y0 = vt.value_to_y(v0);
            let x1 = vt.time_to_x(t1);
            let y1 = vt.value_to_y(v1);

            draw_list
                .add_line([x0, y0], [x1, y1], ghost_color)
                .thickness(1.5)
                .build();
        }
    }
}

pub(in crate::platform::ui::curve_editor) fn draw_suggestion_curve_overlay(
    draw_list: &imgui::DrawListMut,
    overlays: &[SuggestionOverlay],
    visible_curves: &HashSet<PropertyType>,
    vt: &ViewTransform,
) {
    if overlays.is_empty() {
        return;
    }

    for overlay in overlays {
        if !visible_curves.contains(&overlay.property_type) {
            continue;
        }

        if overlay.confidence < 0.05 {
            continue;
        }

        let alpha = if overlay.confidence > 0.8 {
            0.7
        } else if overlay.confidence > 0.3 {
            0.45
        } else {
            0.25
        };
        let ghost_color = if overlay.confidence > 0.8 {
            [0.3, 1.0, 0.3, alpha]
        } else if overlay.confidence > 0.3 {
            [1.0, 1.0, 0.3, alpha]
        } else {
            [1.0, 0.7, 0.3, alpha]
        };

        let kf_x = vt.time_to_x(overlay.time);
        let kf_y = vt.value_to_y(overlay.value);
        let diamond_size = 6.0;

        draw_list
            .add_line(
                [kf_x, kf_y - diamond_size],
                [kf_x + diamond_size, kf_y],
                ghost_color,
            )
            .thickness(2.0)
            .build();
        draw_list
            .add_line(
                [kf_x + diamond_size, kf_y],
                [kf_x, kf_y + diamond_size],
                ghost_color,
            )
            .thickness(2.0)
            .build();
        draw_list
            .add_line(
                [kf_x, kf_y + diamond_size],
                [kf_x - diamond_size, kf_y],
                ghost_color,
            )
            .thickness(2.0)
            .build();
        draw_list
            .add_line(
                [kf_x - diamond_size, kf_y],
                [kf_x, kf_y - diamond_size],
                ghost_color,
            )
            .thickness(2.0)
            .build();

        let handle_color = [ghost_color[0], ghost_color[1], ghost_color[2], alpha * 0.7];

        let in_x = vt.time_to_x(overlay.time + overlay.tangent_in.0);
        let in_y = vt.value_to_y(overlay.value + overlay.tangent_in.1);
        draw_list
            .add_line([kf_x, kf_y], [in_x, in_y], handle_color)
            .thickness(1.0)
            .build();
        draw_list
            .add_circle([in_x, in_y], 3.0, handle_color)
            .filled(true)
            .build();

        let out_x = vt.time_to_x(overlay.time + overlay.tangent_out.0);
        let out_y = vt.value_to_y(overlay.value + overlay.tangent_out.1);
        draw_list
            .add_line([kf_x, kf_y], [out_x, out_y], handle_color)
            .thickness(1.0)
            .build();
        draw_list
            .add_circle([out_x, out_y], 3.0, handle_color)
            .filled(true)
            .build();
    }
}
