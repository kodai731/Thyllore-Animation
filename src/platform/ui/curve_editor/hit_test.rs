use crate::animation::editable::{
    curve_sample, BezierHandle, EditableKeyframe, InterpolationType, KeyframeId, PropertyCurve,
    PropertyType,
};
use crate::ecs::resource::{CurveSelectedKeyframe, TangentHandleType};

use super::view::{find_drawn_curve, ViewTransform};

pub(super) const KEYFRAME_HIT_RADIUS: f32 = 8.0;
pub(super) const CURVE_HIT_RADIUS_PX: f32 = 6.0;

pub(super) fn find_keyframe_time(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
) -> Option<f32> {
    for (curve, _, _) in curves {
        if curve.property_type == *property_type {
            return curve.get_keyframe(keyframe_id).map(|kf| kf.time);
        }
    }
    None
}

pub(super) fn find_keyframe_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> Option<(PropertyType, KeyframeId, f32, f32)> {
    for (curve, _, _) in curves {
        let curve_vt = vt.for_curve(curve);
        for kf in &curve.keyframes {
            let x = curve_vt.time_to_x(kf.time);
            let y = curve_vt.value_to_y(kf.value);

            let dx = mouse_pos[0] - x;
            let dy = mouse_pos[1] - y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= KEYFRAME_HIT_RADIUS {
                return Some((curve.property_type.clone(), kf.id, kf.time, kf.value));
            }
        }
    }

    None
}

pub(super) fn find_curve_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> Option<PropertyType> {
    let mouse_time = vt.x_to_time(mouse_pos[0]);

    curves
        .iter()
        .filter_map(|(curve, _, _)| {
            let value = curve_sample(curve, mouse_time)?;
            let distance = (vt.for_curve(curve).value_to_y(value) - mouse_pos[1]).abs();
            (distance <= CURVE_HIT_RADIUS_PX).then_some((curve.property_type, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(property_type, _)| property_type)
}

pub(super) const TANGENT_HANDLE_HIT_RADIUS: f32 = 10.0;

fn tangent_handle_hit(
    curve_vt: &ViewTransform,
    kf: &EditableKeyframe,
    handle: &BezierHandle,
    mouse_pos: [f32; 2],
) -> bool {
    let hx = curve_vt.time_to_x(kf.time + handle.time_offset);
    let hy = curve_vt.value_to_y(kf.value + handle.value_offset);
    let dx = mouse_pos[0] - hx;
    let dy = mouse_pos[1] - hy;
    (dx * dx + dy * dy).sqrt() <= TANGENT_HANDLE_HIT_RADIUS
}

pub(super) fn find_tangent_handle_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
    vt: &ViewTransform,
) -> Option<(TangentHandleType, PropertyType, KeyframeId, BezierHandle)> {
    for selected in selected_keyframes {
        let Some((curve, _, _)) = find_drawn_curve(curves, selected.property_type) else {
            continue;
        };
        let Some(kf) = curve.get_keyframe(selected.keyframe_id) else {
            continue;
        };
        if kf.interpolation != InterpolationType::Bezier {
            continue;
        }

        let curve_vt = vt.for_curve(curve);
        if tangent_handle_hit(&curve_vt, kf, &kf.in_tangent, mouse_pos) {
            return Some((
                TangentHandleType::In,
                curve.property_type,
                kf.id,
                kf.in_tangent.clone(),
            ));
        }

        if tangent_handle_hit(&curve_vt, kf, &kf.out_tangent, mouse_pos) {
            return Some((
                TangentHandleType::Out,
                curve.property_type,
                kf.id,
                kf.out_tangent.clone(),
            ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::curve_add_keyframe;

    use super::super::test_fixtures::*;

    fn build_flat_curve(property_type: PropertyType, value: f32) -> PropertyCurve {
        let mut curve = PropertyCurve::new(0, property_type);
        curve_add_keyframe(&mut curve, 0.0, value);
        curve_add_keyframe(&mut curve, 1.0, value);
        curve
    }

    #[test]
    fn find_curve_at_position_picks_the_nearest_curve_within_the_radius() {
        let lower = build_flat_curve(PropertyType::TranslationX, 0.2);
        let upper = build_flat_curve(PropertyType::TranslationY, 0.25);
        let curves = [(&lower, [1.0; 4], "x"), (&upper, [1.0; 4], "y")];

        let hit = find_curve_at_position([50.0, 78.0], &curves, &build_pixel_view());

        assert_eq!(hit, Some(PropertyType::TranslationX));
    }

    #[test]
    fn find_curve_at_position_misses_beyond_the_radius() {
        let curve = build_flat_curve(PropertyType::TranslationX, 0.2);
        let curves = [(&curve, [1.0; 4], "x")];

        let hit = find_curve_at_position(
            [50.0, 80.0 - CURVE_HIT_RADIUS_PX - 1.0],
            &curves,
            &build_pixel_view(),
        );

        assert_eq!(hit, None);
    }

    #[test]
    fn normalized_keyframe_hit_uses_each_curve_range() {
        let small = build_two_point_curve(PropertyType::TranslationX, [0.0, 10.0]);
        let large = build_two_point_curve(PropertyType::TranslationY, [100.0, -100.0]);
        let curves = [(&small, [1.0; 4], "x"), (&large, [1.0; 4], "y")];
        let vt = build_normalized_pixel_view();

        let bottom_hit = find_keyframe_at_position([0.0, 100.0], &curves, &vt);
        let top_hit = find_keyframe_at_position([0.0, 0.0], &curves, &vt);

        assert_eq!(
            bottom_hit.map(|hit| (hit.0, hit.3)),
            Some((PropertyType::TranslationX, 0.0))
        );
        assert_eq!(
            top_hit.map(|hit| (hit.0, hit.3)),
            Some((PropertyType::TranslationY, 100.0))
        );
    }
}
