use crate::animation::editable::{curve_add_keyframe, PropertyCurve, PropertyType};
use crate::ecs::resource::CurveValueDisplay;

use super::view::ViewTransform;

pub(super) fn build_unit_view() -> ViewTransform {
    ViewTransform {
        curve_origin: [0.0, 0.0],
        curve_width: 1.0,
        curve_height: 1.0,
        duration: 1.0,
        val_range: 1.0,
        zoom_x: 1.0,
        zoom_y: 1.0,
        view_time_offset: 0.0,
        view_value_offset: 0.0,
        value_display: CurveValueDisplay::Actual,
    }
}

pub(super) fn build_pixel_view() -> ViewTransform {
    ViewTransform {
        curve_width: 100.0,
        curve_height: 100.0,
        ..build_unit_view()
    }
}

pub(super) fn build_normalized_pixel_view() -> ViewTransform {
    ViewTransform {
        val_range: 2.0,
        view_value_offset: -1.0,
        value_display: CurveValueDisplay::Normalized,
        ..build_pixel_view()
    }
}

pub(super) fn build_two_point_curve(
    property_type: PropertyType,
    values: [f32; 2],
) -> PropertyCurve {
    let mut curve = PropertyCurve::new(0, property_type);
    curve_add_keyframe(&mut curve, 0.0, values[0]);
    curve_add_keyframe(&mut curve, 1.0, values[1]);
    curve
}
