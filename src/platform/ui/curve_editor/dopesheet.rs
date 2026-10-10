use crate::animation::editable::{EditableKeyframe, PropertyCurve};
use crate::ecs::resource::CurveSelectedKeyframe;

use super::draw::draw_time_grid;
use super::interaction::KEYFRAME_HIT_RADIUS;
use super::view::ViewTransform;

pub(super) const DOPESHEET_ROW_HEIGHT: f32 = 22.0;
const DIAMOND_HALF_SIZE: f32 = 6.0;
const ROW_BAND_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.04];
const SELECTED_KEY_FILL: [f32; 4] = [0.3, 0.6, 1.0, 1.0];
const SELECTED_KEY_OUTLINE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub(super) fn dopesheet_row_center_y(curve_origin_y: f32, row_index: usize) -> f32 {
    curve_origin_y + DOPESHEET_ROW_HEIGHT * (row_index as f32 + 0.5)
}

fn build_selected_key(curve: &PropertyCurve, kf: &EditableKeyframe) -> CurveSelectedKeyframe {
    CurveSelectedKeyframe {
        property_type: curve.property_type,
        keyframe_id: kf.id,
        original_time: kf.time,
        original_value: kf.value,
    }
}

pub(super) fn find_dopesheet_key(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> Option<CurveSelectedKeyframe> {
    curves
        .iter()
        .enumerate()
        .filter(|(row_index, _)| {
            let row_y = dopesheet_row_center_y(vt.curve_origin[1], *row_index);
            (mouse_pos[1] - row_y).abs() <= KEYFRAME_HIT_RADIUS
        })
        .find_map(|(_, (curve, _, _))| {
            curve
                .keyframes
                .iter()
                .find(|kf| (mouse_pos[0] - vt.time_to_x(kf.time)).abs() <= KEYFRAME_HIT_RADIUS)
                .map(|kf| build_selected_key(curve, kf))
        })
}

pub(super) fn collect_dopesheet_keys_in_rect(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    min: [f32; 2],
    max: [f32; 2],
) -> Vec<CurveSelectedKeyframe> {
    let mut keys = Vec::new();
    for (row_index, (curve, _, _)) in curves.iter().enumerate() {
        let row_y = dopesheet_row_center_y(vt.curve_origin[1], row_index);
        if row_y < min[1] || row_y > max[1] {
            continue;
        }

        for kf in &curve.keyframes {
            let x = vt.time_to_x(kf.time);
            if x >= min[0] && x <= max[0] {
                keys.push(build_selected_key(curve, kf));
            }
        }
    }
    keys
}

pub(super) fn draw_dopesheet(
    draw_list: &imgui::DrawListMut,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    selected_keyframes: &[CurveSelectedKeyframe],
    width: f32,
) {
    let left = vt.curve_origin[0];
    for row_index in (0..curves.len()).step_by(2) {
        let row_top = vt.curve_origin[1] + DOPESHEET_ROW_HEIGHT * row_index as f32;
        draw_list
            .add_rect(
                [left, row_top],
                [left + width, row_top + DOPESHEET_ROW_HEIGHT],
                ROW_BAND_COLOR,
            )
            .filled(true)
            .build();
    }

    draw_time_grid(draw_list, width, vt.curve_height, vt);

    for (row_index, (curve, color, _)) in curves.iter().enumerate() {
        let row_y = dopesheet_row_center_y(vt.curve_origin[1], row_index);
        for kf in &curve.keyframes {
            let center = [vt.time_to_x(kf.time), row_y];
            let is_selected = selected_keyframes
                .iter()
                .any(|s| s.property_type == curve.property_type && s.keyframe_id == kf.id);
            if is_selected {
                draw_key_diamond(draw_list, center, SELECTED_KEY_FILL);
                draw_key_diamond_outline(draw_list, center, SELECTED_KEY_OUTLINE);
            } else {
                draw_key_diamond(draw_list, center, *color);
            }
        }
    }
}

fn diamond_corners(center: [f32; 2]) -> [[f32; 2]; 4] {
    let [x, y] = center;
    [
        [x, y - DIAMOND_HALF_SIZE],
        [x + DIAMOND_HALF_SIZE, y],
        [x, y + DIAMOND_HALF_SIZE],
        [x - DIAMOND_HALF_SIZE, y],
    ]
}

fn draw_key_diamond(draw_list: &imgui::DrawListMut, center: [f32; 2], color: [f32; 4]) {
    let [top, right, bottom, left] = diamond_corners(center);
    draw_list
        .add_triangle(top, right, bottom, color)
        .filled(true)
        .build();
    draw_list
        .add_triangle(top, bottom, left, color)
        .filled(true)
        .build();
}

fn draw_key_diamond_outline(draw_list: &imgui::DrawListMut, center: [f32; 2], color: [f32; 4]) {
    let corners = diamond_corners(center);
    for (index, corner) in corners.iter().enumerate() {
        let next = corners[(index + 1) % corners.len()];
        draw_list
            .add_line(*corner, next, color)
            .thickness(1.5)
            .build();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{curve_add_keyframe, KeyframeId, PropertyType};
    use crate::ecs::resource::CurveValueDisplay;

    fn build_pixel_view(curve_origin_y: f32) -> ViewTransform {
        ViewTransform {
            curve_origin: [0.0, curve_origin_y],
            curve_width: 100.0,
            curve_height: 200.0,
            duration: 1.0,
            val_range: 1.0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            view_time_offset: 0.0,
            view_value_offset: 0.0,
            value_display: CurveValueDisplay::Actual,
        }
    }

    fn build_curve(property_type: PropertyType, times: &[f32]) -> (PropertyCurve, Vec<KeyframeId>) {
        let mut curve = PropertyCurve::new(0, property_type);
        let ids = times
            .iter()
            .map(|&time| curve_add_keyframe(&mut curve, time, 0.0))
            .collect();
        (curve, ids)
    }

    #[test]
    fn row_center_y_steps_by_the_row_height() {
        assert_eq!(dopesheet_row_center_y(100.0, 0), 111.0);
        assert_eq!(dopesheet_row_center_y(100.0, 1), 133.0);
        assert_eq!(dopesheet_row_center_y(100.0, 2), 155.0);
    }

    #[test]
    fn find_dopesheet_key_hits_by_row_and_time() {
        let (first, first_ids) = build_curve(PropertyType::TranslationX, &[0.2, 0.6]);
        let (second, second_ids) = build_curve(PropertyType::TranslationY, &[0.2]);
        let curves = [(&first, [1.0; 4], "x"), (&second, [1.0; 4], "y")];
        let vt = build_pixel_view(0.0);

        let hit = find_dopesheet_key([60.0, 11.0], &curves, &vt).expect("first row key");
        assert_eq!(hit.property_type, PropertyType::TranslationX);
        assert_eq!(hit.keyframe_id, first_ids[1]);

        let hit = find_dopesheet_key([20.0, 33.0], &curves, &vt).expect("second row key");
        assert_eq!(hit.property_type, PropertyType::TranslationY);
        assert_eq!(hit.keyframe_id, second_ids[0]);
    }

    #[test]
    fn find_dopesheet_key_misses_beyond_the_radius_or_outside_the_rows() {
        let (curve, _) = build_curve(PropertyType::TranslationX, &[0.2]);
        let curves = [(&curve, [1.0; 4], "x")];
        let vt = build_pixel_view(0.0);

        assert!(
            find_dopesheet_key([20.0 + KEYFRAME_HIT_RADIUS + 1.0, 11.0], &curves, &vt).is_none()
        );
        assert!(find_dopesheet_key([20.0, 33.0], &curves, &vt).is_none());
    }

    #[test]
    fn collect_dopesheet_keys_in_rect_takes_only_keys_inside_rows_and_times() {
        let (first, first_ids) = build_curve(PropertyType::TranslationX, &[0.2, 0.5, 0.9]);
        let (second, _) = build_curve(PropertyType::TranslationY, &[0.2, 0.5]);
        let curves = [(&first, [1.0; 4], "x"), (&second, [1.0; 4], "y")];
        let vt = build_pixel_view(0.0);

        let keys = collect_dopesheet_keys_in_rect(&curves, &vt, [10.0, 0.0], [60.0, 20.0]);
        let ids: Vec<KeyframeId> = keys.iter().map(|key| key.keyframe_id).collect();
        assert_eq!(ids, vec![first_ids[0], first_ids[1]]);
        assert!(keys
            .iter()
            .all(|key| key.property_type == PropertyType::TranslationX));

        let keys = collect_dopesheet_keys_in_rect(&curves, &vt, [70.0, 0.0], [80.0, 40.0]);
        assert!(keys.is_empty());
    }
}
